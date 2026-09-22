//! Geodesicas cacheadas y transferencia radiativa volumetrica.
//! El cache guarda muestras de mundo, no colores de frames anteriores.
use crate::render::framebuffer::HdrBuffer;
use crate::scene::{
    blackhole::Photon,
    disk::{self, DiskSample, GasTexture},
    relativity::RayFrame,
    stars,
};
use crate::{
    camera::OrbitCamera,
    config,
    math::{noise::NoiseTable, Ray},
};
use glam::{Vec2, Vec3};
use rayon::prelude::*;

#[derive(Clone, Copy, Default)]
struct RaySpan {
    start: usize,
    end: usize,
    background: Vec3,
}
#[derive(Default)]
struct SampleMap {
    rays: Vec<RaySpan>,
    gas: Vec<DiskSample>,
}
impl SampleMap {
    fn shade(&self, index: usize, texture: &GasTexture) -> Vec3 {
        let ray = self.rays[index];
        let mut color = Vec3::ZERO;
        let mut transmittance = 1.0;
        for sample in &self.gas[ray.start..ray.end] {
            let (emission, alpha) = sample.shade(texture);
            color += transmittance * emission * alpha;
            transmittance *= 1.0 - alpha;
            if transmittance < config::MIN_TRANSMITTANCE {
                break;
            }
        }
        color + transmittance * ray.background
    }
}

#[derive(Default)]
pub struct RayCache {
    view: Option<[u32; 10]>,
    samples: Vec<SampleMap>,
}
impl RayCache {
    pub fn render(
        &mut self,
        target: &mut HdrBuffer,
        camera: &OrbitCamera,
        noise: &NoiseTable,
        texture: &GasTexture,
        moving: bool,
    ) -> bool {
        let (width, height) = (target.width(), target.height());
        let key = [
            camera.target.x.to_bits(),
            camera.target.y.to_bits(),
            camera.target.z.to_bits(),
            camera.yaw.to_bits(),
            camera.pitch.to_bits(),
            camera.distance.to_bits(),
            camera.fov_y.to_bits(),
            camera.roll.to_bits(),
            width as u32,
            height as u32,
        ];
        let changed = self.view != Some(key);
        if changed {
            self.samples.clear();
            self.view = Some(key);
        }
        let desired = if moving { 1 } else { config::SPATIAL_SAMPLES };
        if self.samples.len() < desired {
            let index = self.samples.len();
            let projector = camera.projector(width, height);
            let offsets = [
                Vec2::new(-0.25, -0.25),
                Vec2::new(0.25, 0.25),
                Vec2::new(-0.25, 0.25),
                Vec2::new(0.25, -0.25),
            ];
            let offset = offsets[index % offsets.len()];
            // Un vector por fila, sin una asignacion heap por pixel/muestra.
            let rows: Vec<SampleMap> = (0..height)
                .into_par_iter()
                .map(|y| {
                    let mut row = SampleMap {
                        rays: Vec::with_capacity(width),
                        gas: Vec::new(),
                    };
                    for x in 0..width {
                        let ray = trace(projector.ray_for_pixel(x, y, offset), noise, &mut row.gas);
                        row.rays.push(ray);
                    }
                    row
                })
                .collect();
            let total: usize = rows.iter().map(|row| row.gas.len()).sum();
            let mut map = SampleMap {
                rays: Vec::with_capacity(width * height),
                gas: Vec::with_capacity(total),
            };
            for mut row in rows {
                let base = map.gas.len();
                for ray in &mut row.rays {
                    ray.start += base;
                    ray.end += base;
                }
                map.rays.extend(row.rays);
                map.gas.extend(row.gas);
            }
            self.samples.push(map);
        }
        let count = self.samples.len() as f32;
        target
            .pixels_mut()
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, pixel)| {
                let mut color = Vec3::ZERO;
                for map in &self.samples {
                    color += map.shade(i, texture);
                }
                color /= count;
                let peak = color.max_element();
                let ceiling = config::HDR_CEILING / config::EXPOSURE;
                if peak > ceiling {
                    color *= ceiling / peak;
                }
                *pixel = color;
            });
        changed
    }
}

/// Limitar el paso por la escala vertical del gas, sin asintota en y=0.
/// Lejos del disco sigue mandando la precision de la geodesica.
fn volume_step(photon: &Photon) -> f32 {
    let base = photon.step_length();
    let radius = Vec2::new(photon.pos.x, photon.pos.z).length();
    if !(config::DISK_INNER_RADIUS - 1.0..=config::DISK_OUTER_RADIUS + 2.0).contains(&radius) {
        return base;
    }
    let height = radius * config::DISK_HEIGHT_RATIO;
    let speed = photon.vel.length().max(1e-6);
    let atmosphere_height = radius * config::DISK_ATMOSPHERE_HEIGHT_RATIO;
    let base = if photon.pos.y.abs() < atmosphere_height * 4.0 {
        base.min(config::DISK_VOLUME_STEP / speed)
            .min(atmosphere_height * 0.65 / photon.vel.y.abs().max(1e-6))
    } else {
        base
    };
    let distance = photon.pos.y.abs() - height * 3.5;
    if distance > height {
        // Distancia conservadora a la envolvente conica; no saltarse el gas.
        let approach = photon.vel.y.abs() + 3.5 * config::DISK_HEIGHT_RATIO * speed;
        return base.min((distance / approach.max(1e-6) * 0.8).max(config::MIN_STEP));
    }
    base.min(config::DISK_VOLUME_STEP / speed)
        .min(height * 0.65 / photon.vel.y.abs().max(1e-6))
        .max(config::MIN_STEP)
}

fn trace(ray: Ray, noise: &NoiseTable, samples: &mut Vec<DiskSample>) -> RaySpan {
    let mut photon = Photon::from_camera(ray.origin, ray.dir);
    let frame = RayFrame {
        lz_over_e: photon.lz_over_e,
        observer_radius: ray.origin.length(),
    };
    let mut result = RaySpan {
        start: samples.len(),
        ..RaySpan::default()
    };
    let mut optical_depth = 0.0;
    for _ in 0..config::MAX_STEPS {
        if photon.captured() {
            break;
        }
        if photon.escaped() {
            result.background = stars::background(noise, photon.direction(), frame.observer_radius);
            break;
        }
        let previous = photon.pos;
        let step = volume_step(&photon);
        photon.advance(step);
        let midpoint = (previous + photon.pos) * 0.5;
        if let Some(sample) = disk::prepare_volume(midpoint, step, frame) {
            optical_depth += sample.optical_depth;
            samples.push(sample);
            // La modulacion de densidad nunca baja de 0.4. Incluso su minimo
            // deja transmitancia < exp(-20*0.4), debajo del umbral de render.
            if optical_depth > 20.0 {
                break;
            }
        }
    }
    result.end = samples.len();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_tracks_camera_and_keeps_gas_animated() {
        let noise = NoiseTable::new(config::NOISE_SEED);
        let mut camera = OrbitCamera::new();
        let mut cache = RayCache::default();
        let mut gas = GasTexture::new();
        let mut buffer = HdrBuffer::new(64, 36);
        gas.update(&noise, 0.0);
        assert!(cache.render(&mut buffer, &camera, &noise, &gas, false));
        for _ in 1..config::SPATIAL_SAMPLES {
            cache.render(&mut buffer, &camera, &noise, &gas, false);
        }
        let before = buffer.pixels().to_vec();
        gas.update(&noise, 0.25);
        assert!(!cache.render(&mut buffer, &camera, &noise, &gas, false));
        let difference: f32 = before
            .iter()
            .zip(buffer.pixels())
            .map(|(a, b)| (*a - *b).length())
            .sum();
        assert!(difference > 1.0);
        assert_eq!(cache.samples.len(), config::SPATIAL_SAMPLES);
        camera.orbit(0.1, 0.0);
        assert!(cache.render(&mut buffer, &camera, &noise, &gas, false));
        assert_eq!(cache.samples.len(), 1);
    }
}
