//! Buffer de color HDR y su presentacion a minifb.
//!
//! Todo el pipeline trabaja en `Vec3` de `f32` lineal, sin recortar: el disco
//! puede emitir valores muy por encima de 1.0 y eso es justamente lo que
//! alimenta al bloom. Recortar antes de tiempo mata el efecto.
//!
//! La conversion a los `u32` ARGB que espera minifb ocurre en un solo lugar,
//! `present_argb`, y es lo ultimo que pasa en el frame.

use glam::Vec3;
use rayon::prelude::*;

use crate::config;

/// Imagen HDR en punto flotante.
///
/// Guarda dos cosas: los pixeles HDR a resolucion de render, y un buffer LDR a
/// resolucion de ventana. El segundo vive aca adentro porque `present_argb` devuelve
/// una referencia prestada a el; si fuera local a la funcion no habria nada que
/// prestar. Se asigna perezosamente: los buffers que nunca se presentan (el
/// historico de acumulacion, por ejemplo) lo dejan vacio y no gastan memoria.
pub struct HdrBuffer {
    width: usize,
    height: usize,
    pixels: Vec<Vec3>,
    /// Buffer de presentacion, en formato `0x00RRGGBB`. Su tamano es el de la
    /// ventana, no el de `pixels`.
    argb: Vec<u32>,
}

impl HdrBuffer {
    /// Buffer negro de `width x height`.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![Vec3::ZERO; width * height],
            argb: Vec::new(),
        }
    }

    /// Ancho en pixeles del buffer HDR.
    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    /// Alto en pixeles del buffer HDR.
    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }

    /// Acceso de lectura a los pixeles HDR, en row-major.
    #[inline]
    pub fn pixels(&self) -> &[Vec3] {
        &self.pixels
    }

    /// Acceso de escritura a los pixeles HDR, en row-major.
    #[inline]
    pub fn pixels_mut(&mut self) -> &mut [Vec3] {
        &mut self.pixels
    }

    /// Cambia la resolucion del buffer HDR, si hace falta.
    ///
    /// El contenido anterior se pierde. Eso es intencional: un cambio de
    /// resolucion invalida el historico de acumulacion temporal, porque los
    /// pixeles viejos ya no corresponden a los mismos rayos.
    pub fn resize(&mut self, width: usize, height: usize) {
        if self.width == width && self.height == height {
            return;
        }
        self.width = width;
        self.height = height;
        self.pixels.clear();
        self.pixels.resize(width * height, Vec3::ZERO);
    }

    /// Pone todos los pixeles HDR en negro.
    ///
    /// Hoy no la llama nadie: el raymarch escribe todos los pixeles del buffer,
    /// asi que no hace falta limpiar. Queda para cuando alguna etapa acumule en
    /// vez de sobrescribir.
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.pixels.fill(Vec3::ZERO);
    }

    /// Copia el contenido HDR de otro buffer, ajustando la resolucion si difiere.
    pub fn copy_from(&mut self, src: &HdrBuffer) {
        self.resize(src.width, src.height);
        self.pixels.copy_from_slice(&src.pixels);
    }

    /// Convierte a `u32` ARGB y escala al tamano de la ventana.
    ///
    /// Es el unico punto del programa donde el color deja de ser `f32`. Hace dos
    /// cosas en la misma pasada:
    ///
    /// - **Reescalado**: el render corre a `render_scale` del tamano de ventana,
    ///   asi que hay que agrandar. La interpolacion bilineal evita bloques al bajar la resolucion
    ///   durante el movimiento de la camara.
    /// - **Empaquetado**: cada canal pasa a 8 bits.
    ///
    /// Espera que el color ya venga tonemapeado y en `[0, 1]`. Si el tonemap no
    /// corrio, el `saturate` de `pack_argb` recorta y todo se ve quemado.
    pub fn present_argb(&mut self, out_width: usize, out_height: usize) -> &[u32] {
        if self.argb.len() != out_width * out_height {
            self.argb.clear();
            self.argb.resize(out_width * out_height, 0);
        }

        // Se desestructura para que rayon pueda tomar prestados `pixels` (compartido)
        // y `argb` (exclusivo) a la vez: son campos distintos, pero el borrow
        // checker necesita verlo explicito.
        let (pixels, argb) = (&self.pixels, &mut self.argb);
        let (src_w, src_h) = (self.width, self.height);

        // El +0.5 muestrea el centro del pixel de destino, no su esquina: sin
        // eso el reescalado queda corrido medio pixel hacia arriba a la izquierda.
        let scale_x = src_w as f32 / out_width as f32;
        let scale_y = src_h as f32 / out_height as f32;

        argb.par_chunks_mut(out_width)
            .enumerate()
            .for_each(|(y, row)| {
                let sy = ((y as f32 + 0.5) * scale_y - 0.5).clamp(0.0, (src_h - 1) as f32);
                let y0 = sy as usize;
                let y1 = (y0 + 1).min(src_h - 1);
                for (x, out) in row.iter_mut().enumerate() {
                    let sx = ((x as f32 + 0.5) * scale_x - 0.5).clamp(0.0, (src_w - 1) as f32);
                    let x0 = sx as usize;
                    let x1 = (x0 + 1).min(src_w - 1);
                    let top = pixels[y0 * src_w + x0].lerp(pixels[y0 * src_w + x1], sx.fract());
                    let bottom = pixels[y1 * src_w + x0].lerp(pixels[y1 * src_w + x1], sx.fract());
                    *out = pack_argb(top.lerp(bottom, sy.fract()));
                }
            });

        &self.argb
    }
}

/// Empaqueta un color en `[0, 1]` al `0x00RRGGBB` que espera minifb.
///
/// Los NaN se van a 0 por como funciona `clamp` de `f32` con el orden total, asi
/// que un pixel roto se ve negro en vez de basura.
#[inline]
fn pack_argb(color: Vec3) -> u32 {
    let r = (color.x.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    let g = (color.y.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    let b = (color.z.clamp(0.0, 1.0) * 255.0 + 0.5) as u32;
    (r << 16) | (g << 8) | b
}

/// Traduce una escala de render a dimensiones concretas de buffer.
///
/// Acota al piso de `MIN_RENDER_DIMENSION` para que ninguna escala chica termine
/// pidiendo un buffer de ancho o alto cero, que reventaria las divisiones de
/// `present_argb`.
pub fn scaled_dimensions(width: usize, height: usize, scale: f32) -> (usize, usize) {
    let w = ((width as f32 * scale) as usize).max(config::MIN_RENDER_DIMENSION);
    let h = ((height as f32 * scale) as usize).max(config::MIN_RENDER_DIMENSION);
    (w, h)
}
