//! Skybox: cubemap de seis caras con la Via Lactea, generado al iniciar.
//!
//! Cada cara es una textura cuadrada que se llena una sola vez evaluando el
//! cielo en la direccion de cada texel. Despues el render solo la consulta
//! con interpolacion bilineal, en la direccion de ESCAPE de cada geodesica:
//! la banda de la galaxia se curva alrededor de la sombra y forma un anillo
//! de Einstein, igual que las estrellas.
//!
//! El contenido es difuso a proposito (banda, polvo, nebulosas, estrellas no
//! resueltas). Las estrellas puntuales se evaluan aparte por rayo, porque una
//! textura de 768 texels por cara las veria borrosas al acercarse.
//!
//! Los texels se guardan como RGBE de 32 bits: tres mantisas de 8 bits y un
//! exponente comun, el formato de Radiance. Conserva el rango HDR en un cuarto
//! de la memoria de tres `f32`.

use crate::config;
use crate::math::noise::NoiseTable;
use crate::math::{curves, Vec3};
use crate::parallel;

pub struct Skybox {
    size: usize,
    texels: Vec<u32>,
}

impl Skybox {
    pub fn generate(noise: &NoiseTable) -> Self {
        let size = config::SKYBOX_FACE_SIZE;
        let mut texels = vec![0u32; 6 * size * size];
        parallel::chunks_mut(&mut texels, size, 1, |row_index, row| {
            let face = row_index / size;
            let y = row_index % size;
            let v = (y as f32 + 0.5) / size as f32 * 2.0 - 1.0;
            for (x, texel) in row.iter_mut().enumerate() {
                let u = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                *texel = encode(radiance(noise, face_direction(face, u, v)));
            }
        });
        Self { size, texels }
    }

    /// Radiancia del fondo en una direccion de mundo.
    pub fn sample(&self, direction: Vec3) -> Vec3 {
        if direction == Vec3::ZERO {
            return Vec3::ZERO;
        }
        let (face, u, v) = face_uv(direction);
        let n = self.size;
        let x = ((u * 0.5 + 0.5) * n as f32 - 0.5).clamp(0.0, (n - 1) as f32);
        let y = ((v * 0.5 + 0.5) * n as f32 - 0.5).clamp(0.0, (n - 1) as f32);
        let (x0, y0) = (x as usize, y as usize);
        let (x1, y1) = ((x0 + 1).min(n - 1), (y0 + 1).min(n - 1));
        let (fx, fy) = (x.fract(), y.fract());
        let base = face * n * n;
        let texel = |xx: usize, yy: usize| decode(self.texels[base + yy * n + xx]);
        let top = texel(x0, y0).lerp(texel(x1, y0), fx);
        let bottom = texel(x0, y1).lerp(texel(x1, y1), fx);
        top.lerp(bottom, fy)
    }
}

/// Cara dominante y coordenadas en `[-1, 1]`. Ejes de cada cara: X usa (y, z),
/// Y usa (x, z) y Z usa (x, y), con el signo del eje dominante en la cara.
fn face_uv(d: Vec3) -> (usize, f32, f32) {
    let a = d.abs();
    if a.x >= a.y && a.x >= a.z {
        (if d.x > 0.0 { 0 } else { 1 }, d.y / a.x, d.z / a.x)
    } else if a.y >= a.z {
        (if d.y > 0.0 { 2 } else { 3 }, d.x / a.y, d.z / a.y)
    } else {
        (if d.z > 0.0 { 4 } else { 5 }, d.x / a.z, d.y / a.z)
    }
}

/// Inversa de [`face_uv`].
fn face_direction(face: usize, u: f32, v: f32) -> Vec3 {
    let sign = if face.is_multiple_of(2) { 1.0 } else { -1.0 };
    match face / 2 {
        0 => Vec3::new(sign, u, v),
        1 => Vec3::new(u, sign, v),
        _ => Vec3::new(u, v, sign),
    }
    .normalize()
}

/// Cielo en una direccion: banda galactica con bulbo, franjas de polvo,
/// regiones HII rosadas, estrellas no resueltas y nebulosas tenues.
fn radiance(noise: &NoiseTable, d: Vec3) -> Vec3 {
    let pole = config::GALACTIC_POLE.normalize();
    let center = (config::GALACTIC_CENTER - pole * config::GALACTIC_CENTER.dot(pole)).normalize();
    let side = pole.cross(center);
    let latitude = d.dot(pole).clamp(-1.0, 1.0).asin();
    let longitude = d.dot(side).atan2(d.dot(center));

    let bulge = (-(longitude / 0.32).powi(2) - (latitude / 0.12).powi(2)).exp();
    let width = 0.05 + 0.045 * (-(longitude / 0.9).powi(2)).exp();
    // La banda ondula un poco: un plano perfecto se ve artificial.
    let wobble = 0.025 * (longitude * 2.0).sin() + 0.015 * (longitude * 5.0 + 1.3).sin();
    let band = (-((latitude - wobble) / width).powi(2)).exp();
    let halo = (-((latitude - wobble) / (width * 3.2)).powi(2)).exp();

    let warp = Vec3::new(
        noise.value_noise_3d(d * 3.0 + Vec3::new(3.1, 7.7, 1.2)),
        noise.value_noise_3d(d * 3.0 + Vec3::new(9.4, 2.6, 5.8)),
        noise.value_noise_3d(d * 3.0 + Vec3::new(4.9, 8.3, 6.6)),
    );
    let clouds = noise.fbm(d * 7.0 + warp * 0.6, Vec3::ZERO, 6);
    let fine = noise.fbm(d * 38.0 + warp, Vec3::ZERO, 3);
    let grain = (noise.value_noise_3d(d * 300.0) * 0.5 + 0.5).powi(10)
        + (noise.value_noise_3d(d * 140.0 + Vec3::splat(17.0)) * 0.5 + 0.5).powi(12) * 0.6;

    // Polvo: filamentos oscuros concentrados sobre el plano galactico.
    let lane = (-((latitude - wobble - 0.012) / (width * 0.55)).powi(2)).exp();
    let dust_field = noise.fbm(d * 11.0 + warp * 1.4 + Vec3::splat(31.0), Vec3::ZERO, 5);
    let dust = (curves::smoothstep(0.42, 0.66, dust_field) * lane * 0.92
        + curves::smoothstep(0.55, 0.8, fine) * band * 0.35)
        .min(0.95);

    let cool = Vec3::new(0.62, 0.74, 1.0);
    let warm = Vec3::new(1.0, 0.78, 0.52);
    let tint = cool.lerp(warm, bulge.sqrt().min(1.0));
    let light = band * (0.3 + 1.1 * clouds * clouds) + halo * 0.035 + bulge * 1.8;
    let mut color = tint * light * (1.0 - dust) + tint * grain * (band * 3.0 + halo * 0.6);

    let knots = curves::smoothstep(0.7, 0.86, noise.fbm(d * 24.0 + Vec3::splat(5.3), Vec3::ZERO, 3));
    color += Vec3::new(1.0, 0.3, 0.45) * knots * band * 0.9 * (1.0 - dust);

    let nebula = curves::smoothstep(0.56, 0.86, noise.fbm(d * 2.1 + Vec3::splat(41.0), Vec3::ZERO, 5));
    let hue = curves::smoothstep(0.3, 0.7, noise.fbm(d * 1.3 + Vec3::splat(73.0), Vec3::ZERO, 3));
    color += Vec3::new(0.18, 0.32, 0.75).lerp(Vec3::new(0.55, 0.2, 0.62), hue) * nebula * 0.08;

    color * config::SKYBOX_BRIGHTNESS
}

/// Exponente comun y tres mantisas de 8 bits (RGBE de Radiance).
fn encode(c: Vec3) -> u32 {
    let c = c.max(Vec3::ZERO);
    let peak = c.max_element();
    if peak < 1.0e-30 {
        return 0;
    }
    let exponent = peak.log2().floor() as i32 + 1;
    let scale = 256.0 * power_of_two(-exponent);
    let channel = |x: f32| ((x * scale) as u32).min(255);
    (channel(c.x) << 24) | (channel(c.y) << 16) | (channel(c.z) << 8) | (exponent + 128) as u32
}

fn decode(texel: u32) -> Vec3 {
    if texel == 0 {
        return Vec3::ZERO;
    }
    let scale = power_of_two((texel & 0xFF) as i32 - 128 - 8);
    Vec3::new(
        (texel >> 24) as f32,
        ((texel >> 16) & 0xFF) as f32,
        ((texel >> 8) & 0xFF) as f32,
    ) * scale
}

/// `2^e` armando los bits del `f32`, para exponentes normales.
#[inline]
fn power_of_two(e: i32) -> f32 {
    f32::from_bits(((e.clamp(-126, 127) + 127) as u32) << 23)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_round_trip_and_rgbe_keeps_hdr_colors() {
        for face in 0..6 {
            for (u, v) in [(-0.9, 0.3), (0.0, 0.0), (0.7, -0.6)] {
                let (back, bu, bv) = face_uv(face_direction(face, u, v));
                assert_eq!(back, face);
                assert!((bu - u).abs() < 1e-5 && (bv - v).abs() < 1e-5);
            }
        }
        for color in [Vec3::new(0.02, 0.5, 3.0), Vec3::new(1e-4, 2e-4, 3e-5)] {
            let error = (decode(encode(color)) - color).length() / color.length();
            assert!(error < 0.02, "RGBE pierde {error}");
        }
        assert_eq!(decode(encode(Vec3::ZERO)), Vec3::ZERO);
    }

    #[test]
    fn galactic_band_is_brighter_than_the_poles_and_continuous_across_faces() {
        let noise = NoiseTable::new(config::NOISE_SEED);
        let pole = config::GALACTIC_POLE.normalize();
        let center = (config::GALACTIC_CENTER - pole * config::GALACTIC_CENTER.dot(pole)).normalize();
        assert!(radiance(&noise, center).luminance() > 10.0 * radiance(&noise, pole).luminance());
        // A ambos lados de una arista del cubo el cielo es el mismo.
        let a = radiance(&noise, Vec3::new(1.0, 0.2, 0.999_9).normalize());
        let b = radiance(&noise, Vec3::new(0.999_9, 0.2, 1.0).normalize());
        assert!((a - b).length() <= 0.05 * a.length().max(1e-6));
    }
}
