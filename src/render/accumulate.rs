//! Historia corta medida en segundos, no frames; se invalida al mover la camara.
use crate::{config, render::framebuffer::HdrBuffer};
use rayon::prelude::*;

pub fn blend_factor(delta_time: f32) -> f32 {
    if delta_time < 0.0 || !delta_time.is_finite() {
        return 0.0;
    }
    (-delta_time / config::ACCUM_TIME_CONSTANT)
        .exp()
        .min(config::ACCUM_MAX_WEIGHT)
}

pub fn blend(current: &mut HdrBuffer, history: &HdrBuffer, weight: f32) {
    if current.width() != history.width() || current.height() != history.height() {
        return;
    }
    current
        .pixels_mut()
        .par_iter_mut()
        .zip(history.pixels().par_iter())
        .for_each(|(now, before)| *now = now.lerp(*before, weight));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slow_frames_do_not_smear_seconds_of_motion() {
        assert!(blend_factor(0.18) < 0.05);
        assert!(blend_factor(1.0 / 60.0) < 0.8);
        assert_eq!(blend_factor(-1.0), 0.0);
    }
}
