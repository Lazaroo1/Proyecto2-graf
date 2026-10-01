//! Geodesicas cacheadas y transferencia radiativa volumetrica.
//! El cache guarda muestras de mundo, no colores de frames anteriores.
//!
//! En la Endurance un rayo puede terminar en la nave. El impacto guarda su
//! sombreado local y, si el material refleja o deja pasar luz, los indices
//! de los rayos hijos. Los hijos tambien son geodesicas con sus propias
//! muestras de gas, asi que el reflejo del disco en el casco se sigue
//! animando con la camara quieta, igual que el disco visto de frente.
use crate::math::{Vec2, Vec3};
use crate::parallel;
use crate::render::framebuffer::HdrBuffer;
use crate::scene::{
    blackhole::Photon,
    disk::{self, DiskSample, GasTexture},
    endurance::{GlassExit, Ship},
    lighting, material,
    relativity::RayFrame,
    skybox::Skybox,
    stars,
};
use crate::{
    camera::OrbitCamera,
    config,
    math::{noise::NoiseTable, Ray},
};

pub struct SceneFrame<'a> {
    pub noise: &'a NoiseTable,
    pub gas: &'a GasTexture,
    pub sky: stars::SkyFrame,
    pub enhanced: bool,
    /// Fondo con Via Lactea: variante y Endurance.
    pub skybox: Option<&'a Skybox>,
    /// Brillo del skybox; compensa la exposicion de cada version.
    pub sky_gain: f32,
    pub ship: Option<&'a Ship>,
    /// Forma del gas y temperatura de color de esta version.
    pub medium: disk::Medium,
    /// Angulo que cubre un pixel, para el nivel de detalle de las texturas.
    pub pixel_angle: f32,
    /// Barras de cine: solo se trazan las filas de la banda 2.39:1.
    pub letterbox: bool,
    /// Version y giro de la nave, para invalidar el cache cuando cambian.
    pub tag: [u32; 2],
}

impl SceneFrame<'_> {
    /// Cielo en la direccion de escape de una geodesica.
    #[inline(always)]
    fn background(&self, direction: Vec3) -> Vec3 {
        let stars = self.sky.sample(direction);
        match self.skybox {
            Some(skybox) if direction != Vec3::ZERO => {
                stars
                    + skybox.sample(self.sky.rotate(direction))
                        * (self.sky.intensity_shift() * self.sky_gain)
            }
            _ => stars,
        }
    }
}

/// Filas visibles entre las barras de cine, para un buffer `width x height`.
pub fn visible_rows(width: usize, height: usize, letterbox: bool) -> std::ops::Range<usize> {
    if !letterbox {
        return 0..height;
    }
    let visible = ((width as f32 / config::LETTERBOX_ASPECT).round() as usize).min(height);
    let top = (height - visible) / 2;
    top..top + visible
}

/// Indice ausente en `RaySpan::surface` y en los hijos de `Surface`.
const NONE: u32 = u32::MAX;

/// Indices `u32`: 36 bytes por rayo. Se leen todos en cada frame, asi que
/// el tamano pesa en el ancho de banda del sombreado.
#[derive(Clone, Copy)]
struct RaySpan {
    start: u32,
    end: u32,
    background: Vec3,
    escape_direction: Vec3,
    /// Impacto con la nave al final del tramo, o `NONE`.
    surface: u32,
}

impl Default for RaySpan {
    fn default() -> Self {
        Self {
            start: 0,
            end: 0,
            background: Vec3::ZERO,
            escape_direction: Vec3::ZERO,
            surface: NONE,
        }
    }
}

/// Impacto con la nave: luz local fija y pesos de los rayos hijos.
#[derive(Clone, Copy)]
struct Surface {
    color: Vec3,
    reflect_weight: Vec3,
    refract_weight: Vec3,
    reflect: u32,
    refract: u32,
}

#[derive(Default)]
struct SampleMap {
    rays: Vec<RaySpan>,
    gas: Vec<DiskSample>,
    /// Rayos secundarios de reflexion y refraccion.
    children: Vec<RaySpan>,
    surfaces: Vec<Surface>,
}
impl SampleMap {
    fn shade(&self, index: usize, scene: &SceneFrame<'_>) -> Vec3 {
        self.shade_span(&self.rays[index], scene)
    }

    /// Bucle caliente del frame: compone el gas del tramo. Separado del resto
    /// para que la recursion de los rebotes no le quite el inlining.
    #[inline(always)]
    fn composite(&self, ray: &RaySpan, scene: &SceneFrame<'_>) -> (Vec3, f32) {
        let mut color = Vec3::ZERO;
        let mut transmittance = 1.0;
        for sample in &self.gas[ray.start as usize..ray.end as usize] {
            let (emission, alpha) = sample.shade(scene.gas);
            color += transmittance * emission * alpha;
            transmittance *= 1.0 - alpha;
            if transmittance < config::MIN_TRANSMITTANCE {
                break;
            }
        }
        (color, transmittance)
    }

    #[inline(always)]
    fn shade_span(&self, ray: &RaySpan, scene: &SceneFrame<'_>) -> Vec3 {
        let (color, transmittance) = self.composite(ray, scene);
        let background = if ray.surface != NONE {
            if transmittance >= config::MIN_TRANSMITTANCE {
                self.shade_surface(ray.surface, scene)
            } else {
                Vec3::ZERO
            }
        } else if scene.enhanced && transmittance >= config::MIN_TRANSMITTANCE {
            scene.background(ray.escape_direction)
        } else {
            ray.background
        };
        color + transmittance * background
    }

    /// Fuera de linea: solo la Endurance llega aca, y la recursion no debe
    /// inflar el camino de las otras versiones.
    #[inline(never)]
    fn shade_surface(&self, index: u32, scene: &SceneFrame<'_>) -> Vec3 {
        let surface = &self.surfaces[index as usize];
        let mut color = surface.color;
        for (child, weight) in [
            (surface.reflect, surface.reflect_weight),
            (surface.refract, surface.refract_weight),
        ] {
            if child != NONE {
                let span = &self.children[child as usize];
                let (gas, transmittance) = self.composite(span, scene);
                let behind = if span.surface != NONE {
                    if transmittance >= config::MIN_TRANSMITTANCE {
                        self.shade_surface(span.surface, scene)
                    } else {
                        Vec3::ZERO
                    }
                } else if transmittance >= config::MIN_TRANSMITTANCE {
                    scene.background(span.escape_direction)
                } else {
                    Vec3::ZERO
                };
                color += weight * (gas + transmittance * behind);
            }
        }
        color
    }

    fn push_child(&mut self, span: RaySpan) -> u32 {
        self.children.push(span);
        (self.children.len() - 1) as u32
    }

    fn push_surface(&mut self, surface: Surface) -> u32 {
        self.surfaces.push(surface);
        (self.surfaces.len() - 1) as u32
    }
}

#[derive(Default)]
pub struct RayCache {
    view: Option<[u32; 14]>,
    // Conservar las filas evita copiar todo el volumen a una segunda reserva
    // contigua al terminar el trazado, que duplicaria su pico de memoria.
    samples: Vec<Vec<SampleMap>>,
}
impl RayCache {
    pub fn render(
        &mut self,
        target: &mut HdrBuffer,
        camera: &OrbitCamera,
        scene: &SceneFrame<'_>,
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
            scene.enhanced as u32,
            scene.tag[0],
            scene.tag[1],
            scene.letterbox as u32,
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
            let visible = visible_rows(width, height, scene.letterbox);
            // Un vector por fila, sin una asignacion heap por pixel/muestra.
            let mut rows: Vec<SampleMap> = (0..height).map(|_| SampleMap::default()).collect();
            parallel::chunks_mut(&mut rows, 1, 4, |y, rows| {
                let row = &mut rows[0];
                if !visible.contains(&y) {
                    // Detras de las barras de cine no se traza nada.
                    row.rays.resize(width, RaySpan::default());
                    return;
                }
                row.rays.reserve(width);
                for x in 0..width {
                    let ray = trace(projector.ray_for_pixel(x, y, offset), scene, row, 0, 0.0);
                    row.rays.push(ray);
                }
                row.gas.shrink_to_fit();
                row.children.shrink_to_fit();
                row.surfaces.shrink_to_fit();
            });
            self.samples.push(rows);
        }
        let count = self.samples.len() as f32;
        let visible = visible_rows(width, height, scene.letterbox);
        parallel::chunks_mut(target.pixels_mut(), width, 4096, |y, pixels| {
            if !visible.contains(&y) {
                pixels.fill(Vec3::ZERO);
                return;
            }
            for (x, pixel) in pixels.iter_mut().enumerate() {
                let mut color = Vec3::ZERO;
                for map in &self.samples {
                    color += map[y].shade(x, scene);
                }
                color /= count;
                let peak = color.max_element();
                let ceiling = config::HDR_CEILING / config::EXPOSURE;
                if peak > ceiling {
                    color *= ceiling / peak;
                }
                *pixel = color;
            }
        });
        changed
    }
}

/// Limitar el paso por la escala vertical del gas, sin asintota en y=0.
/// Lejos del disco sigue mandando la precision de la geodesica.
#[inline(always)]
fn volume_step(photon: &Photon, medium: disk::Medium) -> f32 {
    let enhanced = medium.enhanced;
    let base = photon.step_length();
    let radius = Vec2::new(photon.pos.x, photon.pos.z).length();
    let outer_radius = if enhanced {
        config::OUTER_GAS_END
    } else {
        config::DISK_OUTER_RADIUS
    };
    if !(config::DISK_INNER_RADIUS - 1.0..=outer_radius + 2.0).contains(&radius) {
        return base;
    }
    // Fuera del disco termico resolver solo el espesor de la envoltura gris;
    // lejos de ambos medios conservar el paso de vacio de la geodesica.
    if enhanced {
        let thermal_distance = (radius - config::DISK_OUTER_RADIUS)
            .max(photon.pos.y.abs() - 3.5 * radius * medium.atmosphere_ratio);
        if thermal_distance > 0.0 {
            let speed = photon.vel.length().max(1e-6);
            let height = radius * medium.outer_ratio;
            let distance = photon.pos.y.abs() - height * 3.5;
            if distance > height {
                let approach = photon.vel.y.abs() + 3.5 * medium.outer_ratio * speed;
                return base.min((distance / approach.max(1e-6) * 0.8).max(config::MIN_STEP));
            }
            return base
                .min(0.4 / speed)
                .min(height * 0.65 / photon.vel.y.abs().max(1e-6))
                .max(config::MIN_STEP);
        }
    }
    let height = radius * config::DISK_HEIGHT_RATIO;
    let speed = photon.vel.length().max(1e-6);
    let atmosphere_height = radius * medium.atmosphere_ratio;
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

/// Integra una geodesica desde `ray`, guardando sus muestras de gas en `row`.
/// `depth` cuenta los rebotes en la nave que llevaron hasta este rayo y
/// `path`, la distancia recorrida desde la camara (para el detalle de texturas).
fn trace(ray: Ray, scene: &SceneFrame<'_>, row: &mut SampleMap, depth: u32, path: f32) -> RaySpan {
    // Dos copias del integrador: la que no conoce la nave no tiene la
    // recursion de los rebotes y conserva el rendimiento de las otras versiones.
    match scene.ship {
        Some(ship) => march::<true>(ray, scene, Some(ship), row, depth, path),
        None => march::<false>(ray, scene, None, row, depth, path),
    }
}

fn march<const SHIP: bool>(
    ray: Ray,
    scene: &SceneFrame<'_>,
    ship: Option<&Ship>,
    row: &mut SampleMap,
    depth: u32,
    path: f32,
) -> RaySpan {
    let mut photon = Photon::from_camera(ray.origin, ray.dir);
    let frame = RayFrame {
        lz_over_e: photon.lz_over_e,
        observer_radius: ray.origin.length(),
    };
    let mut result = RaySpan {
        start: row.gas.len() as u32,
        ..RaySpan::default()
    };
    let mut optical_depth = 0.0;
    for _ in 0..config::MAX_STEPS {
        if photon.captured() {
            break;
        }
        if photon.escaped() {
            result.escape_direction = photon.direction();
            if !scene.enhanced {
                result.background =
                    stars::background(scene.noise, photon.direction(), frame.observer_radius);
            }
            break;
        }
        let previous = photon.pos;
        let step = volume_step(&photon, scene.medium);
        photon.advance(step);
        // Si la cuerda del paso toca la nave, el gas se integra solo hasta
        // el impacto.
        let hit = if SHIP {
            ship.and_then(|ship| ship.intersect(previous, photon.pos).map(|t| (ship, t)))
        } else {
            None
        };
        let (fraction, end) = match hit {
            Some((_, t)) => (t, previous + (photon.pos - previous) * t),
            None => (1.0, photon.pos),
        };
        let midpoint = (previous + end) * 0.5;
        let sample = disk::prepare(midpoint, step * fraction, frame, scene.medium);
        if let Some(sample) = sample {
            optical_depth += sample.optical_depth;
            if !scene.enhanced || sample.optical_depth >= 1e-6 {
                row.gas.push(sample);
            }
            // La modulacion de densidad nunca baja de 0.4. Incluso su minimo
            // deja transmitancia < exp(-20*0.4), debajo del umbral de render.
            if optical_depth > 20.0 {
                break;
            }
        }
        if SHIP {
            if let Some((ship, _)) = hit {
                // El tramo termina antes de lanzar hijos: sus muestras quedan
                // contiguas y los hijos agregan las suyas despues.
                result.end = row.gas.len() as u32;
                let view = (photon.pos - previous).normalize_or_zero();
                let travelled = path + (end - ray.origin).length();
                result.surface = shade_hit(ship, end, view, scene, row, depth, travelled);
                return result;
            }
        }
    }
    result.end = row.gas.len() as u32;
    result
}

/// Sombrea un impacto con la nave y traza sus rayos de reflexion y refraccion.
/// Fuera de linea para que la recursion no cambie el codigo de `trace` en
/// las versiones sin nave.
#[cold]
#[inline(never)]
fn shade_hit(
    ship: &Ship,
    hit: Vec3,
    view: Vec3,
    scene: &SceneFrame<'_>,
    row: &mut SampleMap,
    depth: u32,
    path: f32,
) -> u32 {
    // Tamano del pixel en el punto de impacto: decide cuanto detalle de
    // textura se puede ver sin parpadeo.
    let footprint = path * scene.pixel_angle;
    let shading = ship.shade(scene.noise, hit, view, true, footprint);
    let mut surface = Surface {
        color: shading.color,
        reflect_weight: shading.reflect_weight,
        refract_weight: shading.refract_weight,
        reflect: NONE,
        refract: NONE,
    };
    if depth >= config::SHIP_MAX_BOUNCES {
        return row.push_surface(surface);
    }
    let n = shading.normal;
    if shading.reflect_weight.max_element() > 0.01 {
        let direction = material::reflect(view, n);
        let origin = hit + n * ship.ray_offset();
        let child = trace(Ray::new(origin, direction), scene, row, depth + 1, path);
        surface.reflect = row.push_child(child);
    }
    if shading.refract_weight.max_element() > 0.01 {
        let inside = material::refract(view, n, 1.0 / shading.ior);
        match inside.map(|direction| ship.through_glass(hit, direction)) {
            Some(GlassExit::Out {
                origin,
                direction,
                absorption,
            }) => {
                let child = trace(Ray::new(origin, direction), scene, row, depth + 1, path);
                surface.refract = row.push_child(child);
                surface.refract_weight = surface.refract_weight * absorption;
            }
            Some(GlassExit::Floor {
                point,
                direction,
                absorption,
            }) => {
                // A traves del vidrio se ve el tablero: un impacto opaco sin
                // gas en el camino.
                let floor = ship.shade(scene.noise, point, direction, false, footprint);
                let index = row.push_surface(Surface {
                    color: floor.color,
                    reflect_weight: Vec3::ZERO,
                    refract_weight: Vec3::ZERO,
                    reflect: NONE,
                    refract: NONE,
                });
                let span = RaySpan {
                    start: row.gas.len() as u32,
                    end: row.gas.len() as u32,
                    surface: index,
                    ..RaySpan::default()
                };
                surface.refract = row.push_child(span);
                surface.refract_weight = surface.refract_weight * absorption;
            }
            Some(GlassExit::Lost) | None => surface.refract_weight = Vec3::ZERO,
        }
    }
    row.push_surface(surface)
}

/// Radiancia que llega a `origin` desde `count` direcciones de Fibonacci.
///
/// Es lo que veria una camara puesta en la nave: geodesicas completas, con
/// la lente, el Doppler y la absorcion del gas. Solo cuenta el gas: el cielo
/// es despreciable frente al disco y sus estrellas puntuales meterian ruido.
pub fn capture_environment(scene: &SceneFrame<'_>, origin: Vec3, count: usize) -> Vec<Vec3> {
    const CHUNK: usize = 32;
    let mut radiance = vec![Vec3::ZERO; count];
    parallel::chunks_mut(&mut radiance, CHUNK, 1, |chunk_index, chunk| {
        let mut row = SampleMap::default();
        for (j, value) in chunk.iter_mut().enumerate() {
            row.gas.clear();
            let direction = lighting::fibonacci(chunk_index * CHUNK + j, count);
            let span = march::<false>(Ray::new(origin, direction), scene, None, &mut row, 0, 0.0);
            *value = row.composite(&span, scene).0;
        }
    });
    radiance
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sky_animates_through_cached_geodesics_without_moving_the_gas() {
        let noise = NoiseTable::new(config::NOISE_SEED);
        let mut gas = GasTexture::new();
        gas.update_preview(&noise, 0.0);
        let camera = OrbitCamera::new();
        let mut cache = RayCache::default();
        let mut buffer = HdrBuffer::new(128, 72);
        let mut scene = SceneFrame {
            sky: stars::SkyFrame::new(0.0, camera.position().length()),
            ..plain(&noise, &gas, true)
        };
        for _ in 0..config::SPATIAL_SAMPLES {
            cache.render(&mut buffer, &camera, &scene, false);
        }
        let before = buffer.pixels().to_vec();
        scene.sky = stars::SkyFrame::new(2.0, camera.position().length());
        assert!(!cache.render(&mut buffer, &camera, &scene, false));
        assert_eq!(cache.samples.len(), config::SPATIAL_SAMPLES);
        let change: f32 = before
            .iter()
            .zip(buffer.pixels())
            .map(|(a, b)| (*a - *b).length())
            .sum();
        assert!(change > 0.1, "the sky is still frozen: {change}");
        gas.update(&noise, 2.0);
        let scene = snapshot(&noise, &gas);
        assert!(cache.render(&mut buffer, &camera, &scene, false));
        assert_eq!(cache.samples.len(), 1);
    }

    #[test]
    fn stars_use_bent_escape_rays_and_cannot_shine_through_a_captured_ray() {
        let noise = NoiseTable::new(config::NOISE_SEED);
        let gas = GasTexture::new();
        let scene = SceneFrame {
            sky: stars::SkyFrame::new(0.0, 30.0),
            ..plain(&noise, &gas, true)
        };
        let initial = Ray::new(Vec3::Z * 30.0, Vec3::new(0.11, 0.12, -1.0));
        let escaped = trace(initial, &scene, &mut SampleMap::default(), 0, 0.0);
        assert!(escaped.escape_direction.length() > 0.99);
        assert!(escaped.escape_direction.dot(initial.dir) < 0.99);
        let captured = trace(
            Ray::new(Vec3::Y * 30.0, Vec3::NEG_Y),
            &scene,
            &mut SampleMap::default(),
            0,
            0.0,
        );
        assert_eq!(captured.escape_direction, Vec3::ZERO);
        assert_eq!(scene.sky.sample(captured.escape_direction), Vec3::ZERO);
    }
    fn plain<'a>(noise: &'a NoiseTable, gas: &'a GasTexture, enhanced: bool) -> SceneFrame<'a> {
        SceneFrame {
            noise,
            gas,
            enhanced,
            sky: stars::SkyFrame::new(0.0, 24.0),
            skybox: None,
            sky_gain: 1.0,
            ship: None,
            medium: if enhanced {
                disk::Medium::VARIANT
            } else {
                disk::Medium::ORIGINAL
            },
            pixel_angle: 1.0e-3,
            letterbox: false,
            tag: [0, 0],
        }
    }
    fn snapshot<'a>(noise: &'a NoiseTable, gas: &'a GasTexture) -> SceneFrame<'a> {
        plain(noise, gas, false)
    }

    #[test]
    fn rays_that_hit_the_ship_spawn_curved_reflections_and_refractions() {
        let noise = NoiseTable::new(config::NOISE_SEED);
        let mut gas = GasTexture::new();
        gas.update_preview(&noise, 0.0);
        let center = crate::scene::endurance::ship_center();
        let samples = capture_environment(&plain(&noise, &gas, true), center, 512);
        assert!(samples.iter().any(|value| value.max_element() > 0.0), "el disco ilumina la nave");
        let ship = Ship::new(0.0, crate::scene::lighting::Lighting::from_samples(&samples));
        let scene = SceneFrame {
            ship: Some(&ship),
            ..plain(&noise, &gas, true)
        };
        let mut row = SampleMap::default();
        let camera = OrbitCamera::preset(crate::version::Version::Endurance, 2);
        let origin = camera.position();
        let mut hits = 0;
        let mut reflections = 0;
        for i in 0..64 {
            // Abanico de rayos sobre la nave desde una camara cercana.
            let offset = Vec3::new((i % 8) as f32 - 3.5, (i / 8) as f32 - 3.5, 0.0)
                * (0.32 * config::SHIP_SCALE);
            let span = trace(Ray::new(origin, ship.center() + offset - origin), &scene, &mut row, 0, 0.0);
            if span.surface != NONE {
                hits += 1;
                let surface = row.surfaces[span.surface as usize];
                if surface.reflect != NONE {
                    reflections += 1;
                    let color = row.shade_surface(span.surface, &scene);
                    assert!(color.x.is_finite() && color.min_element() >= 0.0);
                }
            }
        }
        assert!(hits > 8, "la nave debe tapar parte del abanico: {hits}");
        assert!(reflections > 0, "el casco debe reflejar");
        // Las versiones sin nave no producen impactos.
        let mut empty = SampleMap::default();
        let span = trace(Ray::new(origin, ship.center() - origin), &plain(&noise, &gas, true), &mut empty, 0, 0.0);
        assert_eq!(span.surface, NONE);
        assert!(visible_rows(960, 540, true).len() < 540);
        assert_eq!(visible_rows(960, 540, false), 0..540);
    }
    #[test]
    fn cache_tracks_camera_and_keeps_gas_animated() {
        let noise = NoiseTable::new(config::NOISE_SEED);
        let mut camera = OrbitCamera::new();
        let mut cache = RayCache::default();
        let mut gas = GasTexture::new();
        let mut buffer = HdrBuffer::new(64, 36);
        gas.update(&noise, 0.0);
        assert!(cache.render(&mut buffer, &camera, &snapshot(&noise, &gas), false));
        for _ in 1..config::SPATIAL_SAMPLES {
            cache.render(&mut buffer, &camera, &snapshot(&noise, &gas), false);
        }
        let before = buffer.pixels().to_vec();
        gas.update(&noise, 0.25);
        assert!(!cache.render(&mut buffer, &camera, &snapshot(&noise, &gas), false));
        let difference: f32 = before
            .iter()
            .zip(buffer.pixels())
            .map(|(a, b)| (*a - *b).length())
            .sum();
        assert!(difference > 1.0);
        assert_eq!(cache.samples.len(), config::SPATIAL_SAMPLES);
        camera.orbit(0.1, 0.0);
        assert!(cache.render(&mut buffer, &camera, &snapshot(&noise, &gas), false));
        assert_eq!(cache.samples.len(), 1);
    }
}
