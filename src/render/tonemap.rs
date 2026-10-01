//! Compresion HDR con cromaticidad conservada. Sin tinte por canal.
//!
//! La Endurance usa en cambio una presentacion de pelicula: la curva ACES por
//! canal lleva los brillos extremos hacia el blanco como una emulsion, apaga
//! un poco la saturacion y agrega vineta, grano y barras 2.39:1.
use crate::math::Vec3;
use crate::parallel;
use crate::render::framebuffer::HdrBuffer;
use crate::{config, math::curves, render::raymarch};

pub fn apply(buffer: &mut HdrBuffer, exposure: f32, gamma: f32) {
    parallel::chunks_mut(buffer.pixels_mut(), 4096, 16384, |_, pixels| {
        for color in pixels {
            *color = gamma_correct(tonemap_curve(*color * exposure), gamma);
        }
    });
}

/// Presentacion cinematografica de la Endurance. `frame` cambia el grano.
pub fn apply_film(buffer: &mut HdrBuffer, exposure: f32, gamma: f32, letterbox: bool, frame: u32) {
    let (width, height) = (buffer.width(), buffer.height());
    let visible = raymarch::visible_rows(width, height, letterbox);
    let aspect = width as f32 / height as f32;
    parallel::chunks_mut(buffer.pixels_mut(), width, 16384, |y, row| {
        if !visible.contains(&y) {
            row.fill(Vec3::ZERO);
            return;
        }
        let v = (y as f32 + 0.5) / height as f32 * 2.0 - 1.0;
        for (x, color) in row.iter_mut().enumerate() {
            let u = ((x as f32 + 0.5) / width as f32 * 2.0 - 1.0) * aspect;
            let radius = (u * u + v * v) / (aspect * aspect + 1.0);
            let vignette = 1.0 - config::FILM_VIGNETTE * curves::smoothstep(0.12, 1.0, radius);
            let mapped = film_curve(*color * exposure);
            let shown = gamma_correct(mapped * vignette, gamma);
            // Grano de luminancia, mas visible en los medios tonos.
            let grain = (grain_noise(x as u32, y as u32, frame) - 0.5) * config::FILM_GRAIN;
            let weight = 0.4 + 0.6 * (1.0 - (shown.luminance() * 2.0 - 1.0).abs());
            *color = (shown + Vec3::splat(grain * weight)).max(Vec3::ZERO);
        }
    });
}

/// Curva de pelicula: ACES sobre el canal maximo conserva el tono del disco;
/// solo los brillos extremos se mezclan con ACES por canal y se vuelven
/// blancos, como un nucleo sobreexpuesto en la emulsion. Despues, un viraje
/// leve: sombras hacia el cian y luces hacia el ambar.
fn film_curve(exposed: Vec3) -> Vec3 {
    let exposed = exposed.max(Vec3::ZERO);
    let peak = exposed.max_element();
    if peak <= 0.0 {
        return Vec3::ZERO;
    }
    let mapped_peak = aces(peak);
    let hue = exposed * (mapped_peak / peak);
    let channels = Vec3::new(aces(exposed.x), aces(exposed.y), aces(exposed.z));
    let white = curves::smoothstep(0.7, 1.0, mapped_peak) * 0.75;
    let mapped = hue.lerp(channels, white);
    let tone = mapped.luminance();
    // Emulsion de pelicula: colores apagados, nunca tan saturados como el
    // cuerpo negro puro.
    let muted = Vec3::splat(tone).lerp(mapped, config::FILM_SATURATION);
    let grade = Vec3::new(0.94, 0.99, 1.05).lerp(Vec3::new(1.02, 1.0, 0.96), curves::smoothstep(0.05, 0.6, tone));
    (muted * grade).min(Vec3::ONE)
}

/// Ajuste de Narkowicz a la curva ACES: hombro suave hacia el blanco.
#[inline]
fn aces(x: f32) -> f32 {
    let x = x.max(0.0);
    ((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14)).clamp(0.0, 1.0)
}

/// Ruido blanco por pixel y frame, en `[0, 1)`.
#[inline]
fn grain_noise(x: u32, y: u32, frame: u32) -> f32 {
    let mut h = x.wrapping_mul(0x8DA6_B343) ^ y.wrapping_mul(0xD816_3841) ^ frame.wrapping_mul(0xCB1A_B31F);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5BD1_E995);
    h ^= h >> 15;
    (h >> 8) as f32 / 16_777_216.0
}

pub fn tonemap_curve(color: Vec3) -> Vec3 {
    let color = color.max(Vec3::ZERO);
    // Escala comun a R,G,B: no convierte todo lo brillante en blanco.
    color / (1.0 + color.max_element())
}

pub fn gamma_correct(color: Vec3, gamma: f32) -> Vec3 {
    Vec3::new(
        color.x.powf(1.0 / gamma),
        color.y.powf(1.0 / gamma),
        color.z.powf(1.0 / gamma),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn highlights_keep_the_doppler_chromaticity() {
        let input = Vec3::new(3.0, 4.0, 6.0);
        let output = tonemap_curve(input);
        assert!((output.z / output.x - 2.0).abs() < 1e-6);
        assert!(output.max_element() < 1.0);
        assert_eq!(tonemap_curve(Vec3::ZERO), Vec3::ZERO);
    }

    #[test]
    fn film_curve_is_monotonic_and_letterbox_rows_stay_black() {
        let mut previous = 0.0;
        for i in 1..200 {
            let value = aces(i as f32 * 0.05);
            assert!(value >= previous && value <= 1.0);
            previous = value;
        }
        let mut buffer = HdrBuffer::new(96, 54);
        buffer.pixels_mut().fill(Vec3::splat(0.3));
        apply_film(&mut buffer, 1.0, 2.2, true, 3);
        assert_eq!(buffer.pixels()[0], Vec3::ZERO);
        assert!(buffer.pixels()[96 * 27 + 48].x > 0.3);
    }
}
