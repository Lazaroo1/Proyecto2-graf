//! Ruido de valor 3D por tabla precomputada, fBm y turbulencia.
//!
//! # Por que una tabla y no un hash en caliente
//!
//! El campo del plasma se regenera en una textura compartida por frame.
//! Una tabla determinista evita calcular hashes en cada esquina del ruido.
//! Los canales contiguos en Z comparten muestras para asegurar continuidad.
//!
//! # Como sale ruido 3D de una tabla 2D
//!
//! Dos trucos combinados:
//!
//! - **La Z entera desplaza el muestreo en XY.** Se suma `(37, 17) * floor(z)`
//!   a la coordenada. Los dos coeficientes son primos y distintos, asi que
//!   slices consecutivos caen en zonas de la tabla sin correlacion entre si.
//! - **Cada texel guarda dos canales.** El primero es el valor del slice `z`, el
//!   segundo el del slice `z + 1`. Interpolar entre ellos con la parte
//!   fraccionaria de Z cierra la tercera dimension. Guardar los dos canales
//!   juntos cuesta una sola lectura bilineal en vez de dos.
//!
//! Bilineal en XY mas lineal en Z: eso es interpolacion trilineal, con un solo
//! acceso a memoria por muestra.

use glam::{Vec2, Vec3};

use crate::config;

/// Tabla de ruido precomputada, compartida por todos los hilos del raymarch.
///
/// Se construye una vez en `Renderer::new` y se pasa por referencia. Es
/// inmutable, asi que rayon la comparte entre hilos sin sincronizacion.
pub struct NoiseTable {
    /// `NOISE_TABLE_SIZE^2` pares de valores en `[0, 1)`, en row-major.
    /// `x` es el canal del slice Z, `y` el del slice Z+1.
    data: Vec<Vec2>,
}

impl NoiseTable {
    /// Llena la tabla con ruido blanco a partir de `seed`.
    ///
    /// El segundo canal DEBE ser el primero del siguiente slice desplazado.
    /// Dos hashes independientes rompen la continuidad en cada Z entero.
    ///
    /// Es determinista: la misma semilla da siempre la misma tabla, asi que la
    /// escena es reproducible entre corridas.
    pub fn new(seed: u32) -> Self {
        let size = config::NOISE_TABLE_SIZE;
        let mut data = Vec::with_capacity(size * size);
        let scalar: Vec<f32> = (0..size * size)
            .map(|i| unit_float(hash_u32((i as u32) ^ seed.wrapping_mul(0x85EB_CA6B))))
            .collect();
        for y in 0..size {
            for x in 0..size {
                data.push(Vec2::new(
                    scalar[y * size + x],
                    scalar[((y + 17) % size) * size + (x + 37) % size],
                ));
            }
        }
        Self { data }
    }

    /// Lee un texel con wrap toroidal.
    ///
    /// El `& mask` exige que `NOISE_TABLE_SIZE` sea potencia de dos, y funciona
    /// igual para indices negativos por complemento a dos.
    #[inline]
    fn texel(&self, x: i32, y: i32) -> Vec2 {
        let size = config::NOISE_TABLE_SIZE;
        let mask = (size - 1) as i32;
        let xi = (x & mask) as usize;
        let yi = (y & mask) as usize;
        self.data[yi * size + xi]
    }

    /// Muestreo bilineal de la tabla, en coordenadas de texel (no normalizadas).
    ///
    /// Equivale a `textureLod(..., 0.0)` con filtrado lineal y wrap repetido.
    #[inline]
    fn sample_bilinear(&self, x: f32, y: f32) -> Vec2 {
        let x0 = x.floor();
        let y0 = y.floor();
        let fx = x - x0;
        let fy = y - y0;
        let ix = x0 as i32;
        let iy = y0 as i32;

        let top = self.texel(ix, iy).lerp(self.texel(ix + 1, iy), fx);
        let bottom = self.texel(ix, iy + 1).lerp(self.texel(ix + 1, iy + 1), fx);
        top.lerp(bottom, fy)
    }

    /// Ruido de valor 3D en `[-1, 1]`, continuo y con derivada continua.
    ///
    /// El rango centrado en cero es lo que permite sumar octavas sin que la
    /// media se corra; quien quiera `[0, 1]` multiplica por 0.5 y suma 0.5.
    #[inline]
    pub fn value_noise_3d(&self, p: Vec3) -> f32 {
        let i = p.floor();
        let mut f = p - i;

        // Suavizado de Hermite antes de muestrear. Sin esto se ven las costuras
        // de la grilla: la interpolacion lineal tiene la derivada discontinua en
        // cada borde de celda y el ojo lo detecta como un enrejado.
        f = f * f * (3.0 - 2.0 * f);

        let slices = self.sample_bilinear(i.x + 37.0 * i.z + f.x, i.y + 17.0 * i.z + f.y);
        -1.0 + 2.0 * (slices.x + (slices.y - slices.x) * f.z)
    }

    /// Ruido browniano fraccional: **suma** de octavas, cada una al doble de
    /// frecuencia y a la mitad de amplitud. Devuelve `[0, 1]` con media 0.5.
    ///
    /// # Suma y no producto
    ///
    /// La alternativa es multiplicar las octavas en vez de sumarlas. Un producto
    /// de valores en `[0, 1]` produce filamentos dispersos, y para gas dentro de
    /// un volumen eso esta bien. Para modular el brillo de una **superficie** no
    /// sirve, y el motivo es su distribucion: el producto de cinco octavas tiene
    /// media 0.03 y casi toda su masa amontonada cerca de cero. Usado como
    /// multiplicador deja el brillo encerrado en una banda angosta, asi que
    /// oscurece la superficie entera pero practicamente no la modula. El
    /// resultado se ve liso y, peor, **no se nota que se mueve**: rotar un patron
    /// que casi no varia no cambia la imagen.
    ///
    /// La suma reparte su masa alrededor de la media, que es lo que hace falta
    /// para que haya zonas claras y oscuras de verdad.
    ///
    /// `drift` desplaza el punto de muestreo alternando el signo entre octavas.
    /// Ese cizallamiento entre escalas es lo que hace que el patron parezca
    /// revolverse y no solo desplazarse en bloque. Es un vector y no un escalar
    /// sobre un eje fijo porque el llamador puede estar usando dos de las tres
    /// coordenadas para embeber un circulo: desplazar una de esas dos deformaria
    /// el circulo y reintroduciria la costura que el embebido evita.
    pub fn fbm(&self, p: Vec3, drift: Vec3, octaves: u32) -> f32 {
        let mut sum = 0.0;
        let mut amplitude = 0.5;
        let mut frequency = 1.0;
        let mut norm = 0.0;
        let mut q = p;

        for octave in 0..octaves {
            q += if octave % 2 == 0 { drift } else { -drift };
            sum += amplitude * self.value_noise_3d(q * frequency);
            norm += amplitude;
            frequency *= 2.0;
            amplitude *= 0.5;
        }

        // De [-norm, norm] a [0, 1].
        0.5 + 0.5 * sum / norm.max(1.0e-6)
    }
}

/// Hash entero de 32 bits (variante del finalizador de MurmurHash3).
///
/// Solo se usa para construir la tabla, nunca dentro del raymarch.
#[inline]
fn hash_u32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 13;
    x = x.wrapping_mul(0xC2B2_AE35);
    x ^= x >> 16;
    x
}

/// Convierte un hash a `[0, 1)`.
///
/// Descarta los 8 bits bajos: son los menos uniformes del hash y un `f32` no los
/// puede representar todos igual.
#[inline]
fn unit_float(h: u32) -> f32 {
    (h >> 8) as f32 / 16_777_216.0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn noise_is_continuous_across_all_lattice_faces() {
        let noise = NoiseTable::new(12345);
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            for boundary in [-3.0, 0.0, 1.0, 255.0, 256.0] {
                let mut point = Vec3::new(0.37, -1.23, 2.71);
                if axis.x > 0.0 {
                    point.x = boundary;
                }
                if axis.y > 0.0 {
                    point.y = boundary;
                }
                if axis.z > 0.0 {
                    point.z = boundary;
                }
                let left = noise.value_noise_3d(point - axis * 0.0001);
                let right = noise.value_noise_3d(point + axis * 0.0001);
                assert!((left - right).abs() < 0.001, "discontinuous at {point}");
            }
        }
    }
}
