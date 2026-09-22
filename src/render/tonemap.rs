//! Compresion HDR con cromaticidad conservada. Sin tinte por canal.
use crate::render::framebuffer::HdrBuffer;
use glam::Vec3;
use rayon::prelude::*;

pub fn apply(buffer: &mut HdrBuffer, exposure: f32, gamma: f32) {
    buffer.pixels_mut().par_iter_mut().for_each(|color| {
        *color = gamma_correct(tonemap_curve(*color * exposure), gamma);
    });
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
}
