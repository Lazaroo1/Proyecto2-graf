//! Disco de espesor finito con transferencia gris emision/absorcion.
//! Estructura vertical gaussiana y turbulencia procedural; no es GRMHD.
use crate::config;
use crate::math::{blackbody, curves, noise::NoiseTable};
use crate::scene::relativity::{self, RayFrame};
use glam::{Vec2, Vec3};
use rayon::prelude::*;

/// Geometria y corrimiento reutilizables mientras la camara esta quieta.
#[derive(Clone, Copy, Debug, Default)]
pub struct DiskSample {
    tex_x: f32,
    tex_y: f32,
    pub emission: Vec3,
    pub optical_depth: f32,
}

pub fn prepare_volume(hit: Vec3, affine_step: f32, ray: RayFrame) -> Option<DiskSample> {
    prepare_medium(hit, affine_step, ray, false)
}

pub fn prepare_preview_volume(hit: Vec3, affine_step: f32, ray: RayFrame) -> Option<DiskSample> {
    prepare_medium(hit, affine_step, ray, true)
}

fn prepare_medium(
    hit: Vec3,
    affine_step: f32,
    ray: RayFrame,
    enhanced: bool,
) -> Option<DiskSample> {
    let radius = Vec2::new(hit.x, hit.z).length();
    let outer_radius = if enhanced {
        config::OUTER_GAS_END
    } else {
        config::DISK_OUTER_RADIUS
    };
    if !(config::DISK_INNER_RADIUS..outer_radius).contains(&radius) {
        return None;
    }
    let height = radius * config::DISK_HEIGHT_RATIO;
    let atmosphere_height = radius * config::DISK_ATMOSPHERE_HEIGHT_RATIO;
    let z = hit.y / height;
    let atmosphere_z = hit.y / atmosphere_height;
    let outer_height = radius * config::OUTER_GAS_HEIGHT_RATIO;
    let outer_z = hit.y / outer_height;
    if atmosphere_z.abs() > 3.5 && (!enhanced || outer_z.abs() > 3.5) {
        return None;
    }
    let emitted = relativity::disk_temperature(radius);
    let shift = relativity::redshift_factor(radius, ray.observer_radius, ray.lz_over_e);
    let temperature_scale = if enhanced {
        config::PREVIEW_DISK_TEMPERATURE / config::DISK_TEMPERATURE_PEAK
    } else {
        1.0
    };
    let observed = emitted * shift * temperature_scale;
    if observed <= 0.0 {
        return None;
    }
    // Aproximacion bolometrica: g^4 se aplica UNA vez. El lugar planckiano
    // aporta cromaticidad; no es integracion espectral de una camara real.
    let brightness = (emitted * shift / config::DISK_TEMPERATURE_PEAK).powi(4);
    let thermal_source = blackbody::planckian_rgb(observed) * brightness * config::DISK_BRIGHTNESS;
    let thermal_density = if atmosphere_z.abs() <= 3.5 {
        opacity(radius)
            * (config::DISK_OPTICAL_DEPTH * (-0.5 * z * z).exp() / height
                + config::DISK_ATMOSPHERE_OPTICAL_DEPTH
                    * (-0.5 * atmosphere_z * atmosphere_z).exp()
                    / atmosphere_height)
    } else {
        0.0
    };
    let (outer_density, outer_source) = if enhanced {
        let ramp = curves::smoothstep(config::OUTER_GAS_START, config::DISK_OUTER_RADIUS, radius);
        let fade =
            1.0 - curves::smoothstep(config::DISK_OUTER_RADIUS, config::OUTER_GAS_END, radius);
        let density =
            config::OUTER_GAS_OPTICAL_DEPTH * ramp * fade * (-0.5 * outer_z * outer_z).exp()
                / outer_height;
        // Luz difusa gris cada vez mas tenue. Es una fuente aproximada de
        // dispersion, no un cuerpo negro frio que se inventa luz visible.
        let source = Vec3::new(0.94, 0.96, 1.0)
            * config::OUTER_GAS_BRIGHTNESS
            * shift.powi(4)
            * (-0.32 * (radius - config::OUTER_GAS_START).max(0.0)).exp();
        (density, source)
    } else {
        (0.0, Vec3::ZERO)
    };
    let density = thermal_density + outer_density;
    if density <= 1e-10 {
        return None;
    }
    Some(DiskSample {
        // Decorrelacion vertical de los remolinos: evita columnas de ruido
        // identico desde la fotosfera hasta la atmosfera.
        tex_x: (hit.z.atan2(hit.x) + hit.y / radius * 0.8).rem_euclid(std::f32::consts::TAU)
            / std::f32::consts::TAU
            * config::GAS_TEXTURE_WIDTH as f32,
        tex_y: ((radius + hit.y * 1.7 - config::DISK_INNER_RADIUS)
            / (outer_radius - config::DISK_INNER_RADIUS)
            * (config::GAS_TEXTURE_HEIGHT - 1) as f32)
            .clamp(0.0, (config::GAS_TEXTURE_HEIGHT - 1) as f32),
        emission: (thermal_source * thermal_density + outer_source * outer_density) / density,
        // d l_em = d lambda / g: energia local del foton en el gas, con
        // energia unitaria en la camara. Absorcion gris en el marco comovil.
        optical_depth: density / 2.506_628 * affine_step / shift,
    })
}

impl DiskSample {
    pub fn shade(self, gas_texture: &GasTexture) -> (Vec3, f32) {
        let gas = gas_texture.sample(self.tex_x, self.tex_y);
        let alpha = 1.0 - (-self.optical_depth * (0.4 + 0.6 * gas)).exp();
        (self.emission * gas, alpha)
    }
}

/// Campo del plasma en coordenadas (azimut, radio), regenerado en CPU.
/// Compartirlo entre rayos evita evaluar el mismo ruido millones de veces.
pub struct GasTexture {
    values: Vec<f32>,
}

impl GasTexture {
    pub fn new() -> Self {
        Self {
            values: vec![1.0; config::GAS_TEXTURE_WIDTH * config::GAS_TEXTURE_HEIGHT],
        }
    }
    pub fn update(&mut self, noise: &NoiseTable, time: f32) {
        self.update_range(noise, time, config::DISK_OUTER_RADIUS);
    }
    pub fn update_preview(&mut self, noise: &NoiseTable, time: f32) {
        self.update_range(noise, time, config::OUTER_GAS_END);
    }
    fn update_range(&mut self, noise: &NoiseTable, time: f32, outer_radius: f32) {
        let width = config::GAS_TEXTURE_WIDTH;
        self.values
            .par_chunks_mut(width)
            .enumerate()
            .for_each(|(y, row)| {
                let radius = config::DISK_INNER_RADIUS
                    + y as f32 / (config::GAS_TEXTURE_HEIGHT - 1) as f32
                        * (outer_radius - config::DISK_INNER_RADIUS);
                for (x, value) in row.iter_mut().enumerate() {
                    let angle = x as f32 / width as f32 * std::f32::consts::TAU;
                    *value = texture(noise, radius, angle, time);
                }
            });
    }
    fn sample(&self, x: f32, y: f32) -> f32 {
        let width = config::GAS_TEXTURE_WIDTH;
        let height = config::GAS_TEXTURE_HEIGHT;
        let x0 = x.floor() as usize % width;
        let x1 = (x0 + 1) % width;
        let y0 = y.floor() as usize;
        let y1 = (y0 + 1).min(height - 1);
        let (fx, fy) = (x.fract(), y.fract());
        let a = self.values[y0 * width + x0] * (1.0 - fx) + self.values[y0 * width + x1] * fx;
        let b = self.values[y1 * width + x0] * (1.0 - fx) + self.values[y1 * width + x1] * fx;
        a * (1.0 - fy) + b * fy
    }
}

fn opacity(radius: f32) -> f32 {
    let inner = curves::smoothstep(
        config::DISK_INNER_RADIUS,
        config::DISK_INNER_RADIUS * config::DISK_INNER_FADE,
        radius,
    );
    let outer =
        1.0 - curves::smoothstep(config::DISK_FADE_RADIUS, config::DISK_OUTER_RADIUS, radius);
    inner * outer
}

/// Cada campo nace y muere con peso CERO y derivada cero. La version anterior
/// hacia exactamente lo contrario y exponia un reinicio brusco cada 2.5 s.
fn texture(noise: &NoiseTable, radius: f32, angle: f32, time: f32) -> f32 {
    let omega = relativity::keplerian_omega(radius) * config::DISK_TIME_SCALE;
    let cycle = time / config::DISK_FLOW_PERIOD;
    let phase_a = cycle.rem_euclid(1.0);
    let phase_b = (cycle + 0.5).rem_euclid(1.0);
    let weight_a = (std::f32::consts::PI * phase_a).sin().powi(2);
    // Acotar la identidad conserva precision del ruido en sesiones largas.
    let field_a = (cycle.floor() * config::DISK_FLOW_STRIDE).rem_euclid(128.0);
    let field_b = ((cycle + 0.5).floor() * config::DISK_FLOW_STRIDE + 91.7).rem_euclid(128.0);
    // +omega: atan2(z,x) decrece al rotar alrededor de +Y.
    // El giro visible coincide con el signo de L_y usado en el Doppler.
    // Edad centrada limita el cizallamiento acumulado.
    let a = layer(
        noise,
        radius,
        angle + omega * (phase_a - 0.5) * config::DISK_FLOW_PERIOD,
        field_a,
    );
    let b = layer(
        noise,
        radius,
        angle + omega * (phase_b - 0.5) * config::DISK_FLOW_PERIOD,
        field_b,
    );
    let field = a * weight_a + b * (1.0 - weight_a);
    // Nudos calientes y canales oscuros sin mesetas por clamp.
    let gas = ((field - 0.5) * config::DISK_TURBULENCE_CONTRAST).exp();
    1.0 + config::DISK_TURBULENCE_AMOUNT * (gas - 1.0)
}

fn layer(noise: &NoiseTable, radius: f32, angle: f32, field: f32) -> f32 {
    let (s, c) = angle.sin_cos();
    let q = Vec3::new(
        c * config::DISK_NOISE_ANGULAR,
        s * config::DISK_NOISE_ANGULAR,
        radius * config::DISK_NOISE_RADIAL,
    ) * config::DISK_NOISE_SCALE
        + Vec3::splat(field);
    // Deformacion de dominio: remolinos en vez de bandas concentricas limpias.
    // El embebido circular elimina la costura de atan2.
    let warp = Vec3::new(
        noise.value_noise_3d(q * 0.65 + Vec3::new(12.7, 3.1, 9.2)),
        noise.value_noise_3d(q * 0.65 + Vec3::new(7.3, 19.2, 4.1)),
        noise.value_noise_3d(q * 0.65 + Vec3::new(2.1, 5.9, 17.4)),
    );
    let clouds = noise.fbm(q + warp * 1.15, Vec3::ZERO, config::DISK_TURBULENCE_OCTAVES);
    let filaments = 1.0 - noise.value_noise_3d(q * 2.8 + warp * 2.0).abs();
    clouds * 0.8 + filaments * 0.2 - 0.035
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn outer_gas_is_gray_and_fades_to_vacuum() {
        let ray = RayFrame {
            observer_radius: 24.0,
            lz_over_e: 0.0,
        };
        let mut previous = f32::INFINITY;
        for radius in [13.2, 14.0, 14.8] {
            let point = Vec3::new(radius, 0.0, 0.0);
            assert!(prepare_volume(point, 0.1, ray).is_none());
            let sample = prepare_preview_volume(point, 0.1, ray).unwrap();
            let source = sample.emission;
            assert!(source.max_element() / source.min_element() < 1.1);
            let contribution = source.length() * sample.optical_depth;
            assert!(contribution < previous);
            previous = contribution;
        }
        assert!(prepare_preview_volume(Vec3::X * config::OUTER_GAS_END, 0.1, ray).is_none());
    }
    #[test]
    fn gas_advects_in_the_same_direction_as_doppler() {
        let noise = NoiseTable::new(config::NOISE_SEED);
        let radius = 5.0;
        let dt = 0.1;
        let delta = relativity::keplerian_omega(radius) * config::DISK_TIME_SCALE * dt;
        let mut correct = 0.0;
        let mut reversed = 0.0;
        for i in 0..128 {
            let angle = i as f32 / 128.0 * std::f32::consts::TAU;
            let before = texture(&noise, radius, angle, 1.1);
            correct += (texture(&noise, radius, angle - delta, 1.1 + dt) - before).powi(2);
            reversed += (texture(&noise, radius, angle + delta, 1.1 + dt) - before).powi(2);
        }
        assert!(
            correct < reversed * 0.5,
            "correct {correct}, reversed {reversed}"
        );
    }

    #[test]
    fn vertical_optical_depth_converges_to_gaussian_column() {
        let radius = 6.0;
        let ray = RayFrame {
            observer_radius: 24.0,
            lz_over_e: 0.0,
        };
        let shift = relativity::redshift_factor(radius, ray.observer_radius, ray.lz_over_e);
        let expected = opacity(radius)
            * (config::DISK_OPTICAL_DEPTH + config::DISK_ATMOSPHERE_OPTICAL_DEPTH)
            / shift;
        for steps in [400, 800] {
            let step = 2.0 / steps as f32;
            let mut column = 0.0;
            for i in 0..steps {
                if let Some(sample) = prepare_volume(
                    Vec3::new(radius, -1.0 + (i as f32 + 0.5) * step, 0.0),
                    step,
                    ray,
                ) {
                    column += sample.optical_depth;
                }
            }
            assert!((column / expected - 1.0).abs() < 0.002);
        }
    }
    #[test]
    fn flow_is_continuous_at_both_layer_births() {
        let noise = NoiseTable::new(config::NOISE_SEED);
        for t in [0.0, 2.5, 5.0, 7.5, 300.0] {
            for r in [3.5, 5.0, 9.0] {
                for angle in [-2.0, 0.3, 2.7] {
                    let a = texture(&noise, r, angle, t - 0.0001);
                    let b = texture(&noise, r, angle, t + 0.0001);
                    assert!(
                        (a - b).abs() / a.max(b).max(1.0) < 0.02,
                        "jump at t={t}: {a} -> {b}"
                    );
                }
            }
        }
    }
    #[test]
    fn azimuth_has_no_seam() {
        let noise = NoiseTable::new(config::NOISE_SEED);
        let a = texture(&noise, 5.0, -std::f32::consts::PI + 1e-5, 1.3);
        let b = texture(&noise, 5.0, std::f32::consts::PI - 1e-5, 1.3);
        assert!((a - b).abs() < 0.002);
    }
}
