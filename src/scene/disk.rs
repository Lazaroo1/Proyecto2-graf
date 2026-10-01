//! Disco de espesor finito con transferencia gris emision/absorcion.
//! Estructura vertical gaussiana y turbulencia procedural; no es GRMHD.
use crate::config;
use crate::math::{blackbody, curves, noise::NoiseTable};
use crate::math::{Vec2, Vec3};
use crate::parallel;
use crate::scene::relativity::{self, RayFrame};

/// Geometria y corrimiento reutilizables mientras la camara esta quieta.
#[derive(Clone, Copy, Debug, Default)]
pub struct DiskSample {
    tex_x: f32,
    tex_y: f32,
    tex_z: f32,
    pub emission: Vec3,
    pub optical_depth: f32,
}

/// Forma del gas de cada version: alturas de las capas relativas al radio,
/// profundidad optica de la atmosfera y temperatura de color de pico.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Medium {
    /// Gas filamentoso con envoltura gris (variante y Endurance).
    pub enhanced: bool,
    pub peak_temperature: f32,
    pub atmosphere_ratio: f32,
    pub atmosphere_depth: f32,
    pub outer_ratio: f32,
}

impl Medium {
    pub const ORIGINAL: Self = Self {
        enhanced: false,
        peak_temperature: config::DISK_TEMPERATURE_PEAK,
        atmosphere_ratio: config::DISK_ATMOSPHERE_HEIGHT_RATIO,
        atmosphere_depth: config::DISK_ATMOSPHERE_OPTICAL_DEPTH,
        outer_ratio: config::OUTER_GAS_HEIGHT_RATIO,
    };
    pub const VARIANT: Self = Self {
        enhanced: true,
        peak_temperature: config::PREVIEW_DISK_TEMPERATURE,
        atmosphere_ratio: config::PREVIEW_ATMOSPHERE_HEIGHT_RATIO,
        atmosphere_depth: config::PREVIEW_ATMOSPHERE_OPTICAL_DEPTH,
        outer_ratio: config::OUTER_GAS_HEIGHT_RATIO,
    };
    /// Mar de nubes mas compacto: la nave vuela rozando su tope y por encima
    /// queda espacio despejado hacia la sombra, como en la pelicula.
    pub const ENDURANCE: Self = Self {
        enhanced: true,
        peak_temperature: config::ENDURANCE_DISK_TEMPERATURE,
        atmosphere_ratio: config::ENDURANCE_ATMOSPHERE_HEIGHT_RATIO,
        atmosphere_depth: config::PREVIEW_ATMOSPHERE_OPTICAL_DEPTH,
        outer_ratio: config::ENDURANCE_OUTER_GAS_HEIGHT_RATIO,
    };
}

pub fn prepare_volume(hit: Vec3, affine_step: f32, ray: RayFrame) -> Option<DiskSample> {
    prepare(hit, affine_step, ray, Medium::ORIGINAL)
}

pub fn prepare_preview_volume(hit: Vec3, affine_step: f32, ray: RayFrame) -> Option<DiskSample> {
    prepare(hit, affine_step, ray, Medium::VARIANT)
}

/// Altura de la atmosfera de cada version, relativa al radio.
pub fn atmosphere_height_ratio(enhanced: bool) -> f32 {
    if enhanced {
        Medium::VARIANT.atmosphere_ratio
    } else {
        Medium::ORIGINAL.atmosphere_ratio
    }
}

/// Muestra del gas en `hit` para el tramo `affine_step` de una geodesica.
pub fn prepare(hit: Vec3, affine_step: f32, ray: RayFrame, medium: Medium) -> Option<DiskSample> {
    let enhanced = medium.enhanced;
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
    let atmosphere_height = radius * medium.atmosphere_ratio;
    let atmosphere_depth = medium.atmosphere_depth;
    let z = hit.y / height;
    let atmosphere_z = hit.y / atmosphere_height;
    let outer_height = radius * medium.outer_ratio;
    let outer_z = hit.y / outer_height;
    if atmosphere_z.abs() > 3.5 && (!enhanced || outer_z.abs() > 3.5) {
        return None;
    }
    let emitted = relativity::disk_temperature(radius);
    let shift = relativity::redshift_factor(radius, ray.observer_radius, ray.lz_over_e);
    let temperature_scale = medium.peak_temperature / config::DISK_TEMPERATURE_PEAK;
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
                + atmosphere_depth * (-0.5 * atmosphere_z * atmosphere_z).exp() / atmosphere_height)
    } else {
        0.0
    };
    let (outer_density, outer_source) = if enhanced {
        let ramp = curves::smoothstep(config::OUTER_GAS_START, config::OUTER_GAS_PEAK, radius);
        let fade = 1.0 - curves::smoothstep(config::OUTER_GAS_FADE, config::OUTER_GAS_END, radius);
        let density =
            config::OUTER_GAS_OPTICAL_DEPTH * ramp * fade * (-0.5 * outer_z * outer_z).exp()
                / outer_height;
        // Luz difusa gris cada vez mas tenue. Es una fuente aproximada de
        // dispersion, no un cuerpo negro frio que se inventa luz visible.
        let cooling =
            curves::smoothstep(config::OUTER_GAS_START, config::DISK_OUTER_RADIUS, radius);
        let tint = Vec3::new(1.0, 0.82, 0.68).lerp(Vec3::new(0.94, 0.96, 1.0), cooling);
        let source = tint
            * config::OUTER_GAS_BRIGHTNESS
            * shift.powi(4)
            * (-config::OUTER_GAS_LIGHT_FALLOFF * (radius - config::OUTER_GAS_START).max(0.0))
                .exp();
        (density, source)
    } else {
        (0.0, Vec3::ZERO)
    };
    // En la periferia la fotosfera cede gradualmente a un medio gris iluminado
    // por el disco. La mezcla conserva la opacidad total y enfria tambien el
    // borde denso, evitando un corte entre un anillo dorado y una nube gris.
    let (thermal_density, outer_density) = if enhanced {
        let cool_fraction =
            curves::smoothstep(config::DISK_FADE_RADIUS, config::DISK_OUTER_RADIUS, radius);
        let cooled = thermal_density * cool_fraction;
        (thermal_density - cooled, outer_density + cooled)
    } else {
        (thermal_density, outer_density)
    };
    let density = thermal_density + outer_density;
    if density <= 1e-10 {
        return None;
    }
    let (texture_width, texture_height, angle, texture_radius) = if enhanced {
        (
            config::PREVIEW_GAS_TEXTURE_WIDTH,
            config::PREVIEW_GAS_TEXTURE_HEIGHT,
            hit.z.atan2(hit.x),
            radius,
        )
    } else {
        (
            config::GAS_TEXTURE_WIDTH,
            config::GAS_TEXTURE_HEIGHT,
            hit.z.atan2(hit.x) + hit.y / radius * 0.8,
            radius + hit.y * 1.7,
        )
    };
    Some(DiskSample {
        // La variante muestrea tambien la altura: no extruye una textura plana
        // por todo el volumen. El campo gira con la velocidad orbital local.
        tex_x: angle.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU
            * texture_width as f32,
        tex_y: ((texture_radius - config::DISK_INNER_RADIUS)
            / (outer_radius - config::DISK_INNER_RADIUS)
            * (texture_height - 1) as f32)
            .clamp(0.0, (texture_height - 1) as f32),
        tex_z: if enhanced {
            ((outer_z / 7.0 + 0.5) * (config::PREVIEW_GAS_TEXTURE_LAYERS - 1) as f32)
                .clamp(0.0, (config::PREVIEW_GAS_TEXTURE_LAYERS - 1) as f32)
        } else {
            0.0
        },
        emission: (thermal_source * thermal_density + outer_source * outer_density) / density,
        // d l_em = d lambda / g: energia local del foton en el gas, con
        // energia unitaria en la camara. Absorcion gris en el marco comovil.
        optical_depth: density / 2.506_628 * affine_step / shift,
    })
}

impl DiskSample {
    pub fn shade(self, gas_texture: &GasTexture) -> (Vec3, f32) {
        let gas = gas_texture.sample(self.tex_x, self.tex_y, self.tex_z);
        let alpha = 1.0 - (-self.optical_depth * (0.4 + 0.6 * gas)).exp();
        (self.emission * gas, alpha)
    }
}

/// Campo del plasma en coordenadas cilindricas, regenerado en CPU.
/// La variante incluye planos de altura interpolados; el original usa uno.
/// Compartirlo entre rayos evita evaluar el mismo ruido millones de veces.
pub struct GasTexture {
    values: Vec<f32>,
    width: usize,
    height: usize,
    layers: usize,
    flow_fields: [FlowField; 2],
}

#[derive(Default)]
struct FlowField {
    seed: Option<f32>,
    values: Vec<f32>,
}

impl GasTexture {
    pub fn new() -> Self {
        Self {
            values: vec![1.0; config::GAS_TEXTURE_WIDTH * config::GAS_TEXTURE_HEIGHT],
            width: config::GAS_TEXTURE_WIDTH,
            height: config::GAS_TEXTURE_HEIGHT,
            layers: 1,
            flow_fields: [FlowField::default(), FlowField::default()],
        }
    }
    pub fn update(&mut self, noise: &NoiseTable, time: f32) {
        self.update_range(noise, time, false);
    }
    pub fn update_preview(&mut self, noise: &NoiseTable, time: f32) {
        self.update_range(noise, time, true);
    }
    fn update_range(&mut self, noise: &NoiseTable, time: f32, enhanced: bool) {
        let (width, height, layers, outer_radius) = if enhanced {
            (
                config::PREVIEW_GAS_TEXTURE_WIDTH,
                config::PREVIEW_GAS_TEXTURE_HEIGHT,
                config::PREVIEW_GAS_TEXTURE_LAYERS,
                config::OUTER_GAS_END,
            )
        } else {
            (
                config::GAS_TEXTURE_WIDTH,
                config::GAS_TEXTURE_HEIGHT,
                1,
                config::DISK_OUTER_RADIUS,
            )
        };
        self.width = width;
        self.height = height;
        self.layers = layers;
        self.values.resize(width * height * layers, 1.0);
        if enhanced {
            self.update_volume(noise, time);
            return;
        }
        parallel::chunks_mut(&mut self.values, width, 4096, |row_index, row| {
            let y = row_index % height;
            let radius = config::DISK_INNER_RADIUS
                + y as f32 / (height - 1) as f32 * (outer_radius - config::DISK_INNER_RADIUS);
            for (x, value) in row.iter_mut().enumerate() {
                let angle = x as f32 / width as f32 * std::f32::consts::TAU;
                *value = texture(noise, radius, angle, time);
            }
        });
    }

    /// Dos campos espaciales se reutilizan durante su ciclo de vida. Cada frame
    /// solo advecta sus coordenadas y mezcla sus valores; el ruido se regenera
    /// cuando nace un campo con peso cero, no en cada muestra temporal.
    fn update_volume(&mut self, noise: &NoiseTable, time: f32) {
        let (width, height, layers) = (self.width, self.height, self.layers);
        let cycle = time / config::DISK_FLOW_PERIOD;
        let phases = [cycle.rem_euclid(1.0), (cycle + 0.5).rem_euclid(1.0)];
        let weight = (std::f32::consts::PI * phases[0]).sin().powi(2);
        let seeds = [
            (cycle.floor() * config::DISK_FLOW_STRIDE).rem_euclid(128.0),
            ((cycle + 0.5).floor() * config::DISK_FLOW_STRIDE + 91.7).rem_euclid(128.0),
        ];
        for (field, seed) in self.flow_fields.iter_mut().zip(seeds) {
            if field.seed == Some(seed) && field.values.len() == self.values.len() {
                continue;
            }
            field.values.resize(self.values.len(), 0.0);
            parallel::chunks_mut(&mut field.values, width, 4096, |row_index, row| {
                let y = row_index % height;
                let z = row_index / height;
                let radius = config::DISK_INNER_RADIUS
                    + y as f32 / (height - 1) as f32
                        * (config::OUTER_GAS_END - config::DISK_INNER_RADIUS);
                let elevation = (z as f32 / (layers - 1) as f32 - 0.5) * 7.0;
                for (x, value) in row.iter_mut().enumerate() {
                    let angle = x as f32 / width as f32 * std::f32::consts::TAU;
                    *value = layer(noise, radius, angle, seed, elevation, true);
                }
            });
            field.seed = Some(seed);
        }
        let fields = &self.flow_fields;
        parallel::chunks_mut(&mut self.values, width, 4096, |row_index, row| {
            let y = row_index % height;
            let radius = config::DISK_INNER_RADIUS
                + y as f32 / (height - 1) as f32
                    * (config::OUTER_GAS_END - config::DISK_INNER_RADIUS);
            let omega = relativity::keplerian_omega(radius) * config::DISK_TIME_SCALE;
            let offsets = phases.map(|phase| {
                (omega * (phase - 0.5) * config::DISK_FLOW_PERIOD / std::f32::consts::TAU
                    * width as f32)
                    .rem_euclid(width as f32)
            });
            // El desplazamiento es uniforme en cada fila. Resolver el modulo
            // una sola vez evita hacerlo dos veces por voxel y por frame.
            // rem_euclid puede redondear a width si el resto negativo es minimo.
            let mut indices = offsets.map(|offset| offset.floor() as usize % width);
            let fractions = offsets.map(f32::fract);
            let start = row_index * width;
            let a_row = &fields[0].values[start..start + width];
            let b_row = &fields[1].values[start..start + width];
            for value in row {
                let next = indices.map(|i| if i + 1 == width { 0 } else { i + 1 });
                let a = a_row[indices[0]] * (1.0 - fractions[0]) + a_row[next[0]] * fractions[0];
                let b = b_row[indices[1]] * (1.0 - fractions[1]) + b_row[next[1]] * fractions[1];
                let field = a * weight + b * (1.0 - weight);
                let gas = ((field - 0.5) * config::PREVIEW_GAS_TURBULENCE_CONTRAST).exp();
                *value = 1.0 + config::DISK_TURBULENCE_AMOUNT * (gas - 1.0);
                indices = next;
            }
        });
    }
    fn sample(&self, x: f32, y: f32, z: f32) -> f32 {
        if self.layers == 1 {
            return self.sample_layer(x, y, 0);
        }
        let z = z.clamp(0.0, (self.layers - 1) as f32);
        let z0 = z.floor() as usize;
        let z1 = (z0 + 1).min(self.layers - 1);
        let a = self.sample_layer(x, y, z0);
        let b = self.sample_layer(x, y, z1);
        a * (1.0 - z.fract()) + b * z.fract()
    }
    fn sample_layer(&self, x: f32, y: f32, layer: usize) -> f32 {
        let width = self.width;
        let height = self.height;
        let base = layer * width * height;
        let x0 = x.floor() as usize % width;
        let x1 = (x0 + 1) % width;
        let y0 = y.floor() as usize;
        let y1 = (y0 + 1).min(height - 1);
        let (fx, fy) = (x.fract(), y.fract());
        let a = self.values[base + y0 * width + x0] * (1.0 - fx)
            + self.values[base + y0 * width + x1] * fx;
        let b = self.values[base + y1 * width + x0] * (1.0 - fx)
            + self.values[base + y1 * width + x1] * fx;
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

/// Campo ecuatorial del modo original.
fn texture(noise: &NoiseTable, radius: f32, angle: f32, time: f32) -> f32 {
    flow_texture(noise, radius, angle, time, 0.0, false)
}

/// Cada campo nace y muere con peso y derivada cero, tambien en altura.
fn flow_texture(
    noise: &NoiseTable,
    radius: f32,
    angle: f32,
    time: f32,
    elevation: f32,
    enhanced: bool,
) -> f32 {
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
        elevation,
        enhanced,
    );
    let b = layer(
        noise,
        radius,
        angle + omega * (phase_b - 0.5) * config::DISK_FLOW_PERIOD,
        field_b,
        elevation,
        enhanced,
    );
    let field = a * weight_a + b * (1.0 - weight_a);
    let contrast = if enhanced {
        config::PREVIEW_GAS_TURBULENCE_CONTRAST
    } else {
        config::DISK_TURBULENCE_CONTRAST
    };
    // Nudos calientes y canales oscuros sin mesetas por clamp.
    let gas = ((field - 0.5) * contrast).exp();
    1.0 + config::DISK_TURBULENCE_AMOUNT * (gas - 1.0)
}

fn layer(
    noise: &NoiseTable,
    radius: f32,
    angle: f32,
    field: f32,
    elevation: f32,
    enhanced: bool,
) -> f32 {
    let (angular_radius, radial_scale, angle, octaves) = if enhanced {
        (
            config::PREVIEW_GAS_NOISE_ANGULAR + elevation * 0.5,
            config::PREVIEW_GAS_NOISE_RADIAL,
            angle + elevation * config::PREVIEW_GAS_VERTICAL_SHEAR,
            config::PREVIEW_GAS_TURBULENCE_OCTAVES,
        )
    } else {
        (
            config::DISK_NOISE_ANGULAR,
            config::DISK_NOISE_RADIAL,
            angle,
            config::DISK_TURBULENCE_OCTAVES,
        )
    };
    let (s, c) = angle.sin_cos();
    // La altura cambia el radio del embebido circular. Junto con azimut y
    // radio del disco forma un dominio 3D continuo y sin costura angular.
    let q = Vec3::new(
        c * angular_radius,
        s * angular_radius,
        radius * radial_scale,
    ) * config::DISK_NOISE_SCALE
        + Vec3::splat(field);
    // Deformacion de dominio: remolinos en vez de bandas concentricas limpias.
    // El embebido circular elimina la costura de atan2.
    let warp = Vec3::new(
        noise.value_noise_3d(q * 0.65 + Vec3::new(12.7, 3.1, 9.2)),
        noise.value_noise_3d(q * 0.65 + Vec3::new(7.3, 19.2, 4.1)),
        noise.value_noise_3d(q * 0.65 + Vec3::new(2.1, 5.9, 17.4)),
    );
    let clouds = noise.fbm(q + warp * 1.15, Vec3::ZERO, octaves);
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
        for radius in [13.2, 15.0, 17.0, 18.8] {
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

    #[test]
    fn volume_texture_has_height_detail_and_survives_mode_switches() {
        let noise = NoiseTable::new(config::NOISE_SEED);
        let mut gas = GasTexture::new();
        gas.update_preview(&noise, 1.1);
        let mut height_difference = 0.0;
        for x in 0..gas.width {
            let a = gas.sample(x as f32, 90.5, 3.0);
            let b = gas.sample(x as f32, 90.5, 5.0);
            height_difference += (a - b).abs();
        }
        assert!(height_difference / gas.width as f32 > 0.05);
        for y in [0.0, 90.5, (gas.height - 1) as f32] {
            for z in [0.0, 2.5, (gas.layers - 1) as f32] {
                let a = gas.sample(gas.width as f32 - 1e-4, y, z);
                let b = gas.sample(1e-4, y, z);
                assert!(a.is_finite() && a >= 0.0);
                assert!((a - b).abs() < 0.005, "costura en y={y}, z={z}");
            }
        }
        let preview = gas.values.clone();
        gas.update(&noise, 1.1);
        let mut original = GasTexture::new();
        original.update(&noise, 1.1);
        assert_eq!(gas.layers, 1);
        assert_eq!(gas.values, original.values);
        gas.update_preview(&noise, 1.1);
        assert_eq!(gas.values, preview);
    }

    #[test]
    fn cached_volume_advects_continuously_and_handles_time_jumps() {
        let noise = NoiseTable::new(config::NOISE_SEED);
        let mut gas = GasTexture::new();
        gas.update_preview(&noise, 1.1);
        let first = gas.values.clone();
        let row = 60;
        let radius = config::DISK_INNER_RADIUS
            + row as f32 / (gas.height - 1) as f32
                * (config::OUTER_GAS_END - config::DISK_INNER_RADIUS);
        let shift = relativity::keplerian_omega(radius) * config::DISK_TIME_SCALE * 0.05
            / std::f32::consts::TAU
            * gas.width as f32;
        gas.update_preview(&noise, 1.15);
        for layer in [1, 4, 7] {
            let mut correct = 0.0;
            let mut reversed = 0.0;
            for x in 0..gas.width {
                let before = first[(layer * gas.height + row) * gas.width + x];
                let at = |direction: f32| {
                    gas.sample(
                        (x as f32 + direction * shift).rem_euclid(gas.width as f32),
                        row as f32,
                        layer as f32,
                    )
                };
                correct += (at(-1.0) - before).powi(2);
                reversed += (at(1.0) - before).powi(2);
            }
            assert!(
                correct < reversed * 0.5,
                "giro del cache invertido en capa {layer}"
            );
        }
        for time in [0.0, 2.5, 5.0] {
            // Intervalo corto para aislar un salto de semilla de la adveccion
            // de los filamentos mas contrastados cerca del borde interno.
            gas.update_preview(&noise, time - 1e-5);
            let before = gas.values.clone();
            gas.update_preview(&noise, time + 1e-5);
            for (&a, &b) in before.iter().zip(&gas.values) {
                assert!(b.is_finite() && b > 0.0);
                assert!(
                    (a - b).abs() / a.max(b).max(1.0) < 0.02,
                    "salto del campo en t={time}: {a} -> {b}"
                );
            }
        }
        // Un salto grande, una pausa y un rebobinado deben ser deterministas.
        gas.update_preview(&noise, 300.0);
        gas.update_preview(&noise, 1.1);
        assert_eq!(gas.values, first);
        gas.update_preview(&noise, 1.1);
        assert_eq!(gas.values, first);
    }

    #[test]
    fn volumetric_flow_is_continuous_and_follows_orbital_rotation() {
        let noise = NoiseTable::new(config::NOISE_SEED);
        let radius = 8.0;
        let dt = 0.05;
        let delta = relativity::keplerian_omega(radius) * config::DISK_TIME_SCALE * dt;
        for elevation in [-2.0, 0.0, 1.25, 3.0] {
            let mut correct = 0.0;
            let mut reversed = 0.0;
            for i in 0..64 {
                let angle = i as f32 / 64.0 * std::f32::consts::TAU;
                let before = flow_texture(&noise, radius, angle, 1.1, elevation, true);
                correct += (flow_texture(&noise, radius, angle - delta, 1.1 + dt, elevation, true)
                    - before)
                    .powi(2);
                reversed +=
                    (flow_texture(&noise, radius, angle + delta, 1.1 + dt, elevation, true)
                        - before)
                        .powi(2);
                for time in [2.5, 5.0] {
                    let a = flow_texture(&noise, radius, angle, time - 1e-4, elevation, true);
                    let b = flow_texture(&noise, radius, angle, time + 1e-4, elevation, true);
                    assert!((a - b).abs() / a.max(b).max(1.0) < 0.02);
                }
            }
            assert!(
                correct < reversed * 0.5,
                "adveccion invertida en altura {elevation}"
            );
        }
    }
}
