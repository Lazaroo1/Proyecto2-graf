//! Bloom: piramide de mips mas blur gaussiano separable.
//!
//! # La cadena
//!
//! Reproduce los buffers B, C y D de la version GLSL:
//!
//! - **B**: se extraen los pixeles mas brillantes que `BLOOM_THRESHOLD` y se
//!   arma una piramide, cada nivel a la mitad del anterior.
//! - **C**: blur horizontal sobre cada nivel.
//! - **D**: blur vertical sobre el resultado.
//!
//! Sumar niveles de distinta escala es lo que da un halo con falloff ancho y
//! suave: un solo blur con radio grande cuesta carisimo y ademas se ve plano,
//! porque le falta el nucleo concentrado que aportan los mips grandes.
//!
//! # Diferencia con el original
//!
//! En GLSL los mips iban empaquetados en un atlas dentro de una sola textura,
//! porque un buffer de Shadertoy es exactamente una textura. Aca eso no compra
//! nada: solo agregaria aritmetica de offsets. Cada nivel es su propio `Vec<Vec3>`.
//!
//! # Por que separable
//!
//! Un gaussiano 2D de radio `r` es el producto de dos 1D. Aplicar primero el
//! horizontal y despues el vertical da el mismo resultado con `2(2r+1)` muestras
//! por pixel en vez de `(2r+1)^2`. Con `r = 4`: 18 contra 81.

use glam::Vec3;
use rayon::prelude::*;

use crate::config;
use crate::render::framebuffer::HdrBuffer;

/// Pesos de luminancia Rec. 709. El ojo es mucho mas sensible al verde, y usar
/// el promedio de canales en su lugar hace que los rojos saturados entren al
/// bloom con mas fuerza de la que deberian.
const LUMA_WEIGHTS: Vec3 = Vec3::new(0.2126, 0.7152, 0.0722);

/// Un nivel de la piramide de bloom.
pub struct MipLevel {
    /// Ancho en pixeles.
    pub width: usize,
    /// Alto en pixeles.
    pub height: usize,
    /// Color HDR, row-major.
    pub pixels: Vec<Vec3>,
}

impl MipLevel {
    /// Nivel negro del tamano dado.
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![Vec3::ZERO; width * height],
        }
    }

    /// Redimensiona descartando el contenido.
    fn resize(&mut self, width: usize, height: usize) {
        if self.width == width && self.height == height {
            return;
        }
        self.width = width;
        self.height = height;
        self.pixels.clear();
        self.pixels.resize(width * height, Vec3::ZERO);
    }

    /// Muestreo bilineal en coordenadas normalizadas `[0, 1]`.
    ///
    /// Con nearest los mips chicos se ven como bloques, y son justamente los que
    /// cubren mas superficie de la pantalla.
    #[inline]
    fn sample(&self, u: f32, v: f32) -> Vec3 {
        let x = u * self.width as f32 - 0.5;
        let y = v * self.height as f32 - 0.5;
        let x0 = x.floor();
        let y0 = y.floor();
        let fx = x - x0;
        let fy = y - y0;

        let ix = x0 as isize;
        let iy = y0 as isize;
        let top = self.texel(ix, iy).lerp(self.texel(ix + 1, iy), fx);
        let bottom = self.texel(ix, iy + 1).lerp(self.texel(ix + 1, iy + 1), fx);
        top.lerp(bottom, fy)
    }

    /// Lee un texel recortando al borde.
    ///
    /// Recorte y no wrap: con wrap, el brillo de un borde de la pantalla
    /// aparecería como halo en el borde opuesto.
    #[inline]
    fn texel(&self, x: isize, y: isize) -> Vec3 {
        let xi = x.clamp(0, self.width as isize - 1) as usize;
        let yi = y.clamp(0, self.height as isize - 1) as usize;
        self.pixels[yi * self.width + xi]
    }
}

/// Piramide de bloom con su buffer intermedio.
///
/// Es duena de sus asignaciones y se reutiliza entre frames: el bloom no deberia
/// pedir memoria en el loop de render.
pub struct BloomChain {
    /// Niveles de la piramide, del mas grande al mas chico.
    mips: Vec<MipLevel>,
    /// Destino del blur horizontal y fuente del vertical. Hace falta porque el
    /// blur no se puede hacer en sitio: cada pixel de salida lee vecinos que ya
    /// habrian sido sobrescritos.
    scratch: Vec<MipLevel>,
    /// Pesos del gaussiano 1D, ya normalizados. Se calculan una vez.
    kernel: Vec<f32>,
}

impl BloomChain {
    /// Arma la piramide para un buffer de render de `base_width x base_height`.
    pub fn new(base_width: usize, base_height: usize) -> Self {
        let mut chain = Self {
            mips: Vec::new(),
            scratch: Vec::new(),
            kernel: gaussian_kernel(config::BLOOM_BLUR_RADIUS, config::BLOOM_BLUR_SIGMA),
        };
        chain.resize(base_width, base_height);
        chain
    }

    /// Reajusta los niveles cuando cambia la resolucion de render.
    ///
    /// Las dimensiones de cada nivel salen de `BLOOM_BASE_SCALE` y de ir
    /// dividiendo por dos, con piso en 1 para que un mip nunca quede vacio.
    pub fn resize(&mut self, base_width: usize, base_height: usize) {
        let mut w = ((base_width as f32 * config::BLOOM_BASE_SCALE) as usize).max(1);
        let mut h = ((base_height as f32 * config::BLOOM_BASE_SCALE) as usize).max(1);

        self.mips
            .resize_with(config::BLOOM_MIP_COUNT, || MipLevel::new(1, 1));
        self.scratch
            .resize_with(config::BLOOM_MIP_COUNT, || MipLevel::new(1, 1));

        for i in 0..config::BLOOM_MIP_COUNT {
            self.mips[i].resize(w, h);
            self.scratch[i].resize(w, h);
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }
    }

    /// Construye la piramide desenfocada a partir del frame HDR.
    ///
    /// `threshold` viene en unidades del buffer, o sea ya dividido por la
    /// exposicion: el bloom no sabe nada del tonemap.
    pub fn process(&mut self, src: &HdrBuffer, threshold: f32) {
        extract_bright(src, &mut self.mips[0], threshold);

        // `split_at_mut` para leer el nivel anterior y escribir el actual en la
        // misma pasada: son elementos distintos del mismo `Vec`, pero el borrow
        // checker necesita que se lo digan.
        for i in 1..self.mips.len() {
            let (anteriores, actuales) = self.mips.split_at_mut(i);
            downsample(&anteriores[i - 1], &mut actuales[0]);
        }

        for i in 0..self.mips.len() {
            blur_horizontal(&self.mips[i], &mut self.scratch[i], &self.kernel);
            blur_vertical(&self.scratch[i], &mut self.mips[i], &self.kernel);
        }
    }

    /// Suma la piramide sobre `target`.
    ///
    /// Los niveles **no** pesan lo mismo. Cada uno es el doble de ancho que el
    /// anterior, y el mas chico, al estar desenfocado sobre una imagen de unas
    /// pocas decenas de pixeles, es practicamente el promedio de toda la
    /// pantalla: sumarlo con el mismo peso que los demas no agrega un halo, le
    /// sube el piso a la imagen entera y el fondo deja de ser negro.
    ///
    /// Con pesos que decaen geometricamente el halo conserva un nucleo definido
    /// alrededor de lo brillante y una cola ancha pero tenue, que es como se
    /// comporta la dispersion en una lente real.
    ///
    /// Los pesos se normalizan para sumar 1, asi que cambiar `BLOOM_MIP_COUNT` o
    /// la caida altera la forma del halo, no su intensidad.
    pub fn composite(&self, target: &mut HdrBuffer, intensity: f32) {
        let (width, height) = (target.width(), target.height());
        let mips = &self.mips;

        let mut weights = [0.0_f32; 16];
        let mut total = 0.0;
        for (i, weight) in weights.iter_mut().enumerate().take(mips.len()) {
            *weight = config::BLOOM_MIP_FALLOFF.powi(i as i32);
            total += *weight;
        }
        let scale = intensity / total.max(1.0e-6);

        target
            .pixels_mut()
            .par_chunks_mut(width)
            .enumerate()
            .for_each(|(y, row)| {
                let v = (y as f32 + 0.5) / height as f32;
                for (x, pixel) in row.iter_mut().enumerate() {
                    let u = (x as f32 + 0.5) / width as f32;
                    let mut glow = Vec3::ZERO;
                    for (mip, weight) in mips.iter().zip(weights.iter()) {
                        glow += mip.sample(u, v) * *weight;
                    }
                    *pixel += glow * scale;
                }
            });
    }
}

/// Copia a `dst` solo lo que pasa el umbral de brillo, reduciendo la resolucion.
///
/// El escalado promedia el bloque de origen completo en vez de tomar un pixel
/// suelto. Con muestreo puntual, el ruido del raymarch hace que el bloom
/// parpadee: un pixel aislado muy brillante entra o sale del umbral segun el
/// jitter del frame.
///
/// El umbral se aplica sobre la luminancia y despues se escala el color entero,
/// no canal por canal. Restarle el umbral a cada canal desatura: un naranja
/// (1.0, 0.5, 0.1) con umbral 0.4 se volveria (0.6, 0.1, 0.0), mucho mas rojo.
fn extract_bright(src: &HdrBuffer, dst: &mut MipLevel, threshold: f32) {
    let (src_w, src_h) = (src.width(), src.height());
    let src_pixels = src.pixels();
    let (dst_w, dst_h) = (dst.width, dst.height);

    dst.pixels
        .par_chunks_mut(dst_w)
        .enumerate()
        .for_each(|(y, row)| {
            let y0 = y * src_h / dst_h;
            let y1 = (((y + 1) * src_h / dst_h).max(y0 + 1)).min(src_h);

            for (x, out) in row.iter_mut().enumerate() {
                let x0 = x * src_w / dst_w;
                let x1 = (((x + 1) * src_w / dst_w).max(x0 + 1)).min(src_w);

                let mut sum = Vec3::ZERO;
                let mut count = 0.0f32;
                for sy in y0..y1 {
                    let base = sy * src_w;
                    for sx in x0..x1 {
                        sum += src_pixels[base + sx];
                        count += 1.0;
                    }
                }

                let average = sum / count.max(1.0);
                let luma = average.dot(LUMA_WEIGHTS);
                *out = average * ((luma - threshold).max(0.0) / luma.max(1.0e-4));
            }
        });
}

/// Reduce `src` a la mitad con un box filter de 2x2.
///
/// Los indices se recortan porque un nivel de lado impar deja el ultimo texel
/// sin pareja.
fn downsample(src: &MipLevel, dst: &mut MipLevel) {
    let (src_w, src_h) = (src.width, src.height);
    let src_pixels = &src.pixels;
    let dst_w = dst.width;

    dst.pixels
        .par_chunks_mut(dst_w)
        .enumerate()
        .for_each(|(y, row)| {
            let y0 = (2 * y).min(src_h - 1) * src_w;
            let y1 = (2 * y + 1).min(src_h - 1) * src_w;

            for (x, out) in row.iter_mut().enumerate() {
                let x0 = (2 * x).min(src_w - 1);
                let x1 = (2 * x + 1).min(src_w - 1);
                *out = (src_pixels[y0 + x0]
                    + src_pixels[y0 + x1]
                    + src_pixels[y1 + x0]
                    + src_pixels[y1 + x1])
                    * 0.25;
            }
        });
}

/// Convolucion 1D del gaussiano a lo ancho.
fn blur_horizontal(src: &MipLevel, dst: &mut MipLevel, kernel: &[f32]) {
    debug_assert_eq!(src.pixels.len(), dst.pixels.len());
    let width = src.width;
    let src_pixels = &src.pixels;
    let radius = (kernel.len() / 2) as isize;
    let last = width as isize - 1;

    dst.pixels
        .par_chunks_mut(width)
        .enumerate()
        .for_each(|(y, row)| {
            let base = y * width;
            for (x, out) in row.iter_mut().enumerate() {
                let mut acc = Vec3::ZERO;
                for (k, &weight) in kernel.iter().enumerate() {
                    let sx = (x as isize + k as isize - radius).clamp(0, last) as usize;
                    acc += src_pixels[base + sx] * weight;
                }
                *out = acc;
            }
        });
}

/// Convolucion 1D del gaussiano a lo alto.
///
/// Se recorre por filas igual que el horizontal, no por columnas: leer `2r+1`
/// filas enteras mantiene los accesos dentro de pocas lineas de cache, mientras
/// que recorrer una columna salta `width` elementos por lectura.
fn blur_vertical(src: &MipLevel, dst: &mut MipLevel, kernel: &[f32]) {
    debug_assert_eq!(src.pixels.len(), dst.pixels.len());
    let (width, height) = (src.width, src.height);
    let src_pixels = &src.pixels;
    let radius = (kernel.len() / 2) as isize;
    let last = height as isize - 1;

    dst.pixels
        .par_chunks_mut(width)
        .enumerate()
        .for_each(|(y, row)| {
            // El acumulador es la fila de destino, asi que hay que limpiarla:
            // trae el contenido del frame anterior.
            row.fill(Vec3::ZERO);
            for (k, &weight) in kernel.iter().enumerate() {
                let sy = (y as isize + k as isize - radius).clamp(0, last) as usize;
                let base = sy * width;
                for (x, out) in row.iter_mut().enumerate() {
                    *out += src_pixels[base + x] * weight;
                }
            }
        });
}

/// Pesos de un gaussiano 1D discreto, normalizados para que sumen 1.
///
/// Largo `2 * radius + 1`. Que sumen exactamente 1 importa: si no, el blur
/// cambia el brillo medio de la imagen ademas de desenfocarla, y el error se
/// multiplica al pasar por los dos ejes y los cinco niveles.
fn gaussian_kernel(radius: usize, sigma: f32) -> Vec<f32> {
    let mut weights = Vec::with_capacity(2 * radius + 1);
    let denom = 2.0 * sigma * sigma;
    for i in 0..=(2 * radius) {
        let x = i as f32 - radius as f32;
        weights.push((-x * x / denom).exp());
    }
    let sum: f32 = weights.iter().sum();
    for w in weights.iter_mut() {
        *w /= sum;
    }
    weights
}
