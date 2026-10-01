//! La nave Endurance: geometria, luz del entorno y optica del vidrio.
//!
//! # Geometria
//!
//! La nave se describe con funciones de distancia con signo (SDF) en un marco
//! local donde el anillo mide 1 y el eje Y apunta hacia adelante, en la
//! direccion de vuelo. Doce modulos en anillo, unidos por tuneles con bridas,
//! cuatro radios dobles hacia el nucleo, la cupula de vidrio atras y el
//! transbordador Ranger acoplado adelante. Los modulos alternan tres tipos:
//! habitat con aislante dorado y ventanas, motor con toberas y tanques, y
//! modulo de energia con un ala de paneles solares.
//!
//! Los modulos, los tuneles y los radios se evaluan por repeticion angular:
//! solo la celda propia y la vecina. Ademas cada region (anillo, radios y
//! nucleo) tiene una cota inferior de distancia: si la cota ya es mayor que
//! lo encontrado, la region no se evalua.
//!
//! # Escala y precision
//!
//! La nave mide centesimas de rs, cerca del limite de precision de un `f32`
//! a 8 rs del agujero. Toda la marcha dentro de la nave se hace en
//! coordenadas relativas a su centro: la resta grande se hace una sola vez por
//! cuerda y despues los numeros son del tamano de la nave.
//!
//! # Luz
//!
//! La iluminacion es la luz real del entorno en la posicion de la nave
//! (`Lighting`): luces clave con sombra suave, irradiancia difusa en armonicos
//! esfericos con oclusion ambiental, brillos GGX y reflejos borrosos para las
//! superficies rugosas.

use crate::config;
use crate::math::noise::NoiseTable;
use crate::math::{curves, Mat3, Vec2, Vec3};
use crate::scene::lighting::Lighting;
use crate::scene::material::{self, MaterialId, Region};

use std::f32::consts::{PI, TAU};

const RING_RADIUS: f32 = 1.0;
const MODULE_COUNT: i32 = 12;
const MODULE_HALF: Vec3 = Vec3::new(0.16, 0.075, 0.1);
const MODULE_ROUND: f32 = 0.008;
const BAND_HALF: Vec3 = Vec3::new(0.055, 0.0805, 0.1055);
const NOZZLE_OFFSET: f32 = 0.075;
const NOZZLE_CENTER: f32 = -0.12;
const NOZZLE_HALF: f32 = 0.045;
const NOZZLE_EXIT: f32 = 0.055;
const NOZZLE_THROAT: f32 = 0.032;
const TANK_RADIUS: f32 = 0.022;
const PANEL_CENTER: f32 = 0.33;
const PANEL_HALF: Vec3 = Vec3::new(0.15, 0.003, 0.2);
const CONNECTOR_RADIUS: f32 = 0.036;
const SPOKE_COUNT: i32 = 4;
const SPOKE_RADIUS: f32 = 0.009;
const HUB_RADIUS: f32 = 0.2;
const HUB_HALF: f32 = 0.1;
const DOME_RADIUS: f32 = 0.15;
/// Radio de la esfera que contiene toda la nave, en unidades locales.
const BOUND: f32 = 1.6;
/// Distancia de impacto y separacion de los rayos secundarios, locales.
const HIT_EPSILON: f32 = 2.5e-4;
const RAY_OFFSET: f32 = 2.5e-3;
/// Mas alla de esta cota una region no se detalla: alcanza con la cota.
const COARSE: f32 = 0.06;

/// Pieza de la nave mas cercana a un punto. El entero es el indice angular
/// del modulo, tunel o radio, sin reducir modulo 12.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Part {
    Body(i32),
    Band(i32),
    Nozzle(i32),
    Tank(i32),
    Panel(i32),
    Mast(i32),
    Equipment(i32),
    Pipe(i32),
    Connector(i32),
    Flange(i32),
    Spoke(i32),
    Hub,
    HubFlange,
    Collar,
    Dome,
    Ranger,
}

/// Resultado del sombreado local de un impacto.
pub struct SurfaceShading {
    /// Emision mas luz directa, difusa y reflejo borroso: lo que no depende
    /// de otros rayos.
    pub color: Vec3,
    pub normal: Vec3,
    pub reflect_weight: Vec3,
    pub refract_weight: Vec3,
    pub ior: f32,
}

/// Salida de un rayo que entro a la cupula.
pub enum GlassExit {
    /// Sale por la superficie curva: sigue como rayo nuevo.
    Out {
        origin: Vec3,
        direction: Vec3,
        absorption: Vec3,
    },
    /// Llega a la base: ve el tablero de instrumentos.
    Floor {
        point: Vec3,
        direction: Vec3,
        absorption: Vec3,
    },
    /// Rebota adentro mas veces de las que se siguen.
    Lost,
}

pub struct Ship {
    center: Vec3,
    rotation: Mat3,
    inverse: Mat3,
    scale: f32,
    bound: f32,
    lighting: Lighting,
}

/// Centro de la nave en mundo. La camara de la Endurance orbita este punto.
pub fn ship_center() -> Vec3 {
    let (sin_az, cos_az) = config::SHIP_AZIMUTH.sin_cos();
    Vec3::new(
        config::SHIP_ORBIT_RADIUS * sin_az,
        config::SHIP_HEIGHT,
        config::SHIP_ORBIT_RADIUS * cos_az,
    )
}

/// Direccion de vuelo: hacia adentro, desviada `SHIP_HEADING` hacia el sentido
/// de giro del disco, con el cabeceo `SHIP_PITCH`.
pub fn ship_forward() -> Vec3 {
    let (sin_az, cos_az) = config::SHIP_AZIMUTH.sin_cos();
    let inward = Vec3::new(-sin_az, 0.0, -cos_az);
    let tangent = Vec3::new(cos_az, 0.0, -sin_az);
    let (sin_h, cos_h) = config::SHIP_HEADING.sin_cos();
    let (sin_p, cos_p) = config::SHIP_PITCH.sin_cos();
    ((inward * cos_h + tangent * sin_h) * cos_p + Vec3::Y * sin_p).normalize()
}

impl Ship {
    /// Nave en su posicion de `config`, con el anillo girado `spin` radianes
    /// alrededor de su eje de vuelo e iluminada por `lighting`.
    pub fn new(spin: f32, lighting: Lighting) -> Self {
        let center = ship_center();
        let forward = ship_forward();
        let side = forward.cross(Vec3::Y).normalize();
        let up = side.cross(forward);
        let (sin_b, cos_b) = config::SHIP_BANK.sin_cos();
        let x = side * cos_b + up * sin_b;
        let z = up * cos_b - side * sin_b;
        let rotation = Mat3::from_columns(x, forward, z) * Mat3::from_rotation_y(spin);
        Self {
            center,
            rotation,
            inverse: rotation.transpose(),
            scale: config::SHIP_SCALE,
            bound: BOUND * config::SHIP_SCALE,
            lighting,
        }
    }

    #[cfg(test)]
    pub fn center(&self) -> Vec3 {
        self.center
    }

    /// Separacion de los rayos secundarios respecto del casco, en mundo.
    pub fn ray_offset(&self) -> f32 {
        RAY_OFFSET * self.scale
    }

    #[inline]
    fn to_local(&self, p: Vec3) -> Vec3 {
        self.inverse * (p - self.center) * (1.0 / self.scale)
    }

    #[inline]
    fn to_world(&self, p: Vec3) -> Vec3 {
        self.rotation * p * self.scale + self.center
    }

    /// Primer impacto en la cuerda `a -> b`, como fraccion de la cuerda.
    pub fn intersect(&self, a: Vec3, b: Vec3) -> Option<f32> {
        let segment = b - a;
        let length = segment.length();
        if length <= 0.0 {
            return None;
        }
        let dir = segment / length;
        let oc = a - self.center;
        let half_b = oc.dot(dir);
        let c = oc.length_squared() - self.bound * self.bound;
        let discriminant = half_b * half_b - c;
        if discriminant < 0.0 {
            return None;
        }
        let root = discriminant.sqrt();
        let exit = (-half_b + root).min(length);
        let enter = (-half_b - root).max(0.0);
        if enter > exit {
            return None;
        }
        // Marcha en unidades locales, desde el origen relativo de la cuerda.
        let origin = self.inverse * oc * (1.0 / self.scale);
        let local_dir = self.inverse * dir;
        let (mut t, limit) = (enter / self.scale, exit / self.scale);
        for _ in 0..config::SHIP_MARCH_STEPS {
            let distance = evaluate(origin + local_dir * t, true).0;
            if distance < HIT_EPSILON {
                return Some(t * self.scale / length);
            }
            t += distance;
            if t > limit {
                return None;
            }
        }
        None
    }

    /// Sombreado local de un impacto. `view` es la direccion del rayo que
    /// llega; `glass` decide si la cupula forma parte de la escena;
    /// `footprint` es el tamano de un pixel en el punto, en mundo.
    pub fn shade(
        &self,
        noise: &NoiseTable,
        hit: Vec3,
        view: Vec3,
        glass: bool,
        footprint: f32,
    ) -> SurfaceShading {
        let p = self.to_local(hit);
        let lod = (footprint / self.scale).max(1.0e-5);
        let mut geometric = normal(p, glass);
        let view_local = self.inverse * view;
        if geometric.dot(view_local) > 0.0 {
            geometric = -geometric;
        }
        let info = surface(p, geometric, glass);
        let material = info.material.material();
        let texture = material.sample(noise, info.uv, info.region, lod);

        // Relieve: el gradiente de la altura inclina la normal en el plano
        // tangente, sin mover la geometria. El paso nunca es menor que el
        // pixel, que es lo que filtra el relieve de lejos.
        let (base_eps, strength) = material.texture.bump();
        let eps = base_eps.max(lod);
        let du = material.sample(noise, info.uv + Vec2::new(eps, 0.0), info.region, lod).height;
        let dv = material.sample(noise, info.uv + Vec2::new(0.0, eps), info.region, lod).height;
        let gradient = Vec2::new(du - texture.height, dv - texture.height) * (strength / eps);
        let tu = (info.tangent_u - geometric * info.tangent_u.dot(geometric)).normalize_or_zero();
        let tv = (info.tangent_v - geometric * info.tangent_v.dot(geometric)).normalize_or_zero();
        let mut n_local = (geometric - tu * gradient.x - tv * gradient.y).normalize_or_zero();
        if n_local.dot(view_local) > 0.0 {
            n_local = geometric;
        }
        let n = self.rotation * n_local;
        let v = -view;

        // Fresnel con color: los metales reflejan con el tono de su albedo.
        let metalness = texture.metalness.clamp(0.0, 1.0);
        let roughness = texture.roughness.clamp(0.02, 1.0);
        let hue = texture.albedo * (1.0 / texture.albedo.max_element().max(1.0e-4));
        let tint = Vec3::ONE.lerp(hue, metalness);
        let f0 = (material.reflectivity * texture.reflectivity).min(1.0);
        let cos_v = n.dot(v).clamp(0.0, 1.0);
        let reflectance = if f0 > 0.0 {
            tint * material::fresnel(cos_v, f0)
        } else {
            Vec3::ZERO
        };
        let kr = reflectance.max_element();
        let transparency = (material.transparency * texture.transparency).clamp(0.0, 1.0);
        let kt = (1.0 - kr) * transparency;
        let kd = (1.0 - kr) * (1.0 - transparency) * (1.0 - metalness);

        // Difuso: entorno en armonicos con oclusion, mas las luces clave con
        // sombra. Brillos GGX de las luces clave, ensanchados por su tamano.
        let occlusion = ambient_occlusion(p, geometric);
        let mut diffuse = self.lighting.irradiance(n) * occlusion;
        let mut highlight = Vec3::ZERO;
        let start = p + geometric * 0.004;
        for key in self.lighting.keys() {
            let n_dot_l = n.dot(key.direction);
            let l_local = self.inverse * key.direction;
            if n_dot_l <= 0.0 || geometric.dot(l_local) <= 0.0 {
                continue;
            }
            let shadow = soft_shadow(start, l_local, key.softness);
            if shadow <= 0.0 {
                continue;
            }
            let irradiance = key.irradiance * shadow;
            diffuse += irradiance * n_dot_l;
            let spread = roughness.max(key.angular_radius.min(0.6));
            let half = (key.direction + v).normalize_or_zero();
            let base = Vec3::splat(f0.max(0.02)).lerp(tint * f0, metalness);
            let grazing = (1.0 - half.dot(v).clamp(0.0, 1.0)).powi(5);
            let fresnel = base + (Vec3::ONE - base) * grazing;
            highlight += irradiance * fresnel * material::ggx(n, v, key.direction, spread);
        }

        // Las superficies rugosas no ven un espejo sino el entorno borroso:
        // esa parte del reflejo se resuelve con los armonicos.
        let rough = curves::smoothstep(0.12, 0.55, roughness);
        let glossy = self.lighting.glossy(material::reflect(view, n)) * occlusion;
        let color = material.emission * texture.emission
            + texture.albedo * diffuse * (kd / PI)
            + highlight * (material.specular * texture.specular)
            + reflectance * glossy * rough;

        SurfaceShading {
            color,
            normal: n,
            reflect_weight: reflectance * (1.0 - rough),
            refract_weight: Vec3::splat(kt),
            ior: material.ior,
        }
    }

    /// Recorre el interior de la cupula desde `entry` con la direccion ya
    /// refractada hacia adentro. Sigue hasta cuatro reflexiones totales.
    pub fn through_glass(&self, entry: Vec3, direction: Vec3) -> GlassExit {
        let ior = MaterialId::Glass.material().ior;
        let mut d = (self.inverse * direction).normalize_or_zero();
        let mut p = self.to_local(entry) + d * 0.003;
        let mut path = 0.0;
        for _ in 0..4 {
            let mut t = 0.0;
            let mut boundary = false;
            for _ in 0..96 {
                let inside = -sd_dome(p + d * t);
                if inside < 1.0e-4 {
                    boundary = true;
                    break;
                }
                t += inside.max(5.0e-4);
                if t > 2.0 * DOME_RADIUS + 0.05 {
                    break;
                }
            }
            if !boundary {
                return GlassExit::Lost;
            }
            p += d * t;
            path += t;
            let absorption = (config::GLASS_ABSORPTION * -path).exp();
            if p.y > -HUB_HALF - 0.002 {
                return GlassExit::Floor {
                    point: self.to_world(Vec3::new(p.x, -HUB_HALF, p.z)),
                    direction: self.rotation * d,
                    absorption,
                };
            }
            let outward = dome_normal(p);
            match material::refract(d, -outward, ior) {
                Some(out) => {
                    return GlassExit::Out {
                        origin: self.to_world(p + outward * 0.003),
                        direction: self.rotation * out,
                        absorption,
                    }
                }
                None => {
                    d = material::reflect(d, -outward);
                    p -= outward * 0.002;
                }
            }
        }
        GlassExit::Lost
    }
}

/// Sombra suave hacia una luz direccional, en unidades locales.
fn soft_shadow(start: Vec3, l: Vec3, softness: f32) -> f32 {
    let mut result: f32 = 1.0;
    let mut t = 0.012;
    for _ in 0..48 {
        let h = evaluate(start + l * t, false).0;
        if h < 1.0e-4 {
            return 0.0;
        }
        result = result.min(softness * h / t);
        t += h.clamp(0.006, 0.3);
        if t > 2.0 * BOUND {
            break;
        }
    }
    let r = result.clamp(0.0, 1.0);
    r * r * (3.0 - 2.0 * r)
}

/// Oclusion ambiental por cinco muestras a lo largo de la normal.
fn ambient_occlusion(p: Vec3, n: Vec3) -> f32 {
    let mut occlusion = 0.0;
    let mut weight = 1.0;
    for i in 1..=5 {
        let h = 0.01 + 0.035 * i as f32;
        occlusion += (h - evaluate(p + n * h, false).0).max(0.0) * weight;
        weight *= 0.62;
    }
    (1.0 - 3.2 * occlusion).clamp(0.08, 1.0)
}

fn normal(p: Vec3, glass: bool) -> Vec3 {
    // Tetraedro: cuatro evaluaciones en vez de seis.
    let e = 2.0e-4;
    let k = [
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(1.0, 1.0, 1.0),
    ];
    k.iter()
        .fold(Vec3::ZERO, |sum, &dir| sum + dir * evaluate(p + dir * e, glass).0)
        .normalize_or_zero()
}

fn dome_normal(p: Vec3) -> Vec3 {
    (p - Vec3::new(0.0, -HUB_HALF, 0.0)).normalize_or_zero()
}

/// Distancia local a la nave y pieza mas cercana.
fn evaluate(p: Vec3, glass: bool) -> (f32, Part) {
    let radial = Vec2::new(p.x, p.z).length();
    let angle = p.z.atan2(p.x);
    // Cotas inferiores por region en el plano (radio, altura). La distancia
    // al eje es 1-Lipschitz, asi que la de un rectangulo en ese plano nunca
    // supera la distancia real a lo que contiene.
    let mut regions = [
        (sd_rect(Vec2::new(radial - 1.21, p.y), Vec2::new(0.35, 0.17)), 0u8),
        (sd_rect(Vec2::new(radial - 0.55, p.y), Vec2::new(0.37, 0.032)), 1),
        (sd_rect(Vec2::new(radial - 0.13, p.y - 0.18), Vec2::new(0.13, 0.45)), 2),
    ];
    regions.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut best = (f32::INFINITY, Part::Hub);
    for (bound, region) in regions {
        if bound >= best.0 {
            break;
        }
        if bound > COARSE {
            best = (bound, Part::Hub);
            break;
        }
        match region {
            0 => ring(p, angle, &mut best),
            1 => spokes(p, angle, &mut best),
            _ => core(p, glass, &mut best),
        }
    }
    best
}

#[inline]
fn keep(best: &mut (f32, Part), d: f32, part: Part) {
    if d < best.0 {
        *best = (d, part);
    }
}

fn ring(p: Vec3, angle: f32, best: &mut (f32, Part)) {
    for k in neighbors(angle, MODULE_COUNT) {
        let q = module_frame(p, k);
        keep(best, sd_round_box(q, MODULE_HALF, MODULE_ROUND), Part::Body(k));
        // Dos tuberias recorren la cara interior de cada modulo.
        let pipe = Vec3::new(q.x - q.x.clamp(-0.15, 0.15), q.y.abs() - 0.04, q.z + 0.107);
        keep(best, pipe.length() - 0.006, Part::Pipe(k));
        match k.rem_euclid(MODULE_COUNT) % 3 {
            0 => {
                keep(best, sd_round_box(q, BAND_HALF, 0.006), Part::Band(k));
                let equipment = q - Vec3::new(0.0, 0.0, 0.112);
                keep(best, sd_round_box(equipment, Vec3::new(0.045, 0.03, 0.012), 0.004), Part::Equipment(k));
            }
            1 => {
                let n = Vec3::new(q.x.abs() - NOZZLE_OFFSET, q.y - NOZZLE_CENTER, q.z);
                keep(best, sd_capped_cone(n, NOZZLE_HALF, NOZZLE_EXIT, NOZZLE_THROAT), Part::Nozzle(k));
                // Tanques de propelente sobre la cara exterior.
                let tank = Vec3::new(q.x - q.x.clamp(-0.1, 0.1), q.y.abs() - 0.035, q.z - 0.122);
                keep(best, tank.length() - TANK_RADIUS, Part::Tank(k));
            }
            _ => {
                let panel = q - Vec3::new(0.0, 0.0, PANEL_CENTER);
                keep(best, sd_round_box(panel, PANEL_HALF, 0.002), Part::Panel(k));
                let mast = q - Vec3::new(0.0, 0.0, 0.14);
                keep(best, sd_round_box(mast, Vec3::new(0.012, 0.012, 0.045), 0.004), Part::Mast(k));
            }
        }
    }
    // Tuneles entre modulos, a medio paso angular, con una brida al centro.
    let step = TAU / MODULE_COUNT as f32;
    for j in neighbors(angle - 0.5 * step, MODULE_COUNT) {
        let q = frame(p, (j as f32 + 0.5) * step) - Vec3::new(0.0, 0.0, RING_RADIUS);
        keep(best, sd_cylinder_x(q, CONNECTOR_RADIUS, 0.13), Part::Connector(j));
        keep(best, sd_cylinder_x(q, 0.05, 0.008), Part::Flange(j));
    }
}

fn spokes(p: Vec3, angle: f32, best: &mut (f32, Part)) {
    for k in neighbors(angle, SPOKE_COUNT) {
        let q = frame(p, k as f32 * TAU / SPOKE_COUNT as f32);
        let along = q.z.clamp(HUB_RADIUS - 0.01, RING_RADIUS - 0.09);
        // Dos tubos paralelos por radio.
        let tube = Vec3::new(q.x, q.y.abs() - 0.02, q.z - along);
        keep(best, tube.length() - SPOKE_RADIUS, Part::Spoke(k));
    }
}

fn core(p: Vec3, glass: bool, best: &mut (f32, Part)) {
    keep(best, sd_rounded_cylinder(p, HUB_RADIUS, HUB_HALF, 0.012), Part::Hub);
    let flange = Vec3::new(p.x, p.y.abs() - 0.055, p.z);
    keep(best, sd_cylinder(flange, 0.214, 0.008), Part::HubFlange);
    keep(best, sd_cylinder(p - Vec3::new(0.0, 0.135, 0.0), 0.085, 0.035), Part::Collar);
    keep(best, sd_ranger(p), Part::Ranger);
    if glass {
        keep(best, sd_dome(p), Part::Dome);
    }
}

/// Transbordador acoplado adelante: fuselaje plano con nariz en punta, alas
/// delta y deriva. Cada pieza es una caja recortada por planos; el maximo de
/// distancias con signo es una cota valida para la marcha.
fn sd_ranger(p: Vec3) -> f32 {
    let q = p - Vec3::new(0.0, 0.38, 0.0);
    let x = q.x.abs();
    let body = sd_round_box(q, Vec3::new(0.05, 0.2, 0.02), 0.012);
    let nose = (0.17 * x + 0.05 * (q.y - 0.24)) / 0.1772;
    let fuselage = body.max(nose);
    let w = q - Vec3::new(0.0, -0.11, -0.008);
    let wing = sd_round_box(w, Vec3::new(0.12, 0.07, 0.004), 0.002);
    let sweep = (0.14 * (x - 0.12) + 0.08 * (w.y + 0.07)) / 0.1612;
    let f = q - Vec3::new(0.0, -0.14, 0.035);
    let fin = sd_round_box(f, Vec3::new(0.004, 0.05, 0.035), 0.002);
    let fin_sweep = 0.6 * f.z + 0.8 * (f.y - 0.02);
    fuselage.min(wing.max(sweep)).min(fin.max(fin_sweep))
}

/// Los dos indices angulares mas cercanos: la celda propia y la vecina.
fn neighbors(angle: f32, count: i32) -> [i32; 2] {
    let step = TAU / count as f32;
    let k = (angle / step).round();
    let other = if angle > k * step { k + 1.0 } else { k - 1.0 };
    [k as i32, other as i32]
}

/// Coordenadas en el marco de un elemento radial: x tangencial, y axial y z
/// radial desde el eje.
#[inline]
fn frame(p: Vec3, theta: f32) -> Vec3 {
    let (s, c) = theta.sin_cos();
    Vec3::new(-p.x * s + p.z * c, p.y, p.x * c + p.z * s)
}

#[inline]
fn module_frame(p: Vec3, k: i32) -> Vec3 {
    frame(p, k as f32 * TAU / MODULE_COUNT as f32) - Vec3::new(0.0, 0.0, RING_RADIUS)
}

/// Cupula hemisferica atras del nucleo (lado -Y), sobre la cara del tablero.
fn sd_dome(p: Vec3) -> f32 {
    let sphere = (p - Vec3::new(0.0, -HUB_HALF, 0.0)).length() - DOME_RADIUS;
    sphere.max(p.y + HUB_HALF)
}

fn sd_rect(p: Vec2, half: Vec2) -> f32 {
    let q = Vec2::new(p.x.abs() - half.x, p.y.abs() - half.y);
    Vec2::new(q.x.max(0.0), q.y.max(0.0)).length() + q.x.max(q.y).min(0.0)
}

fn sd_round_box(p: Vec3, half: Vec3, radius: f32) -> f32 {
    let q = p.abs() - half + Vec3::splat(radius);
    q.max(Vec3::ZERO).length() + q.max_element().min(0.0) - radius
}

fn sd_cylinder(p: Vec3, radius: f32, half: f32) -> f32 {
    let dx = Vec2::new(p.x, p.z).length() - radius;
    let dy = p.y.abs() - half;
    dx.max(dy).min(0.0) + Vec2::new(dx.max(0.0), dy.max(0.0)).length()
}

/// Cilindro con eje X, centrado en el origen.
fn sd_cylinder_x(p: Vec3, radius: f32, half: f32) -> f32 {
    sd_cylinder(Vec3::new(p.y, p.x, p.z), radius, half)
}

fn sd_rounded_cylinder(p: Vec3, radius: f32, half: f32, round: f32) -> f32 {
    sd_cylinder(p, radius - round, half - round) - round
}

/// Cono truncado sobre Y: radio `bottom` en `y = -half` y `top` en `+half`.
fn sd_capped_cone(p: Vec3, half: f32, bottom: f32, top: f32) -> f32 {
    let q = Vec2::new(Vec2::new(p.x, p.z).length(), p.y);
    let k1 = Vec2::new(top, half);
    let k2 = Vec2::new(top - bottom, 2.0 * half);
    let rim = if q.y < 0.0 { bottom } else { top };
    let ca = Vec2::new(q.x - q.x.min(rim), q.y.abs() - half);
    let t = ((k1 - q).dot(k2) / k2.dot(k2)).clamp(0.0, 1.0);
    let cb = q - k1 + k2 * t;
    let sign = if cb.x < 0.0 && ca.y < 0.0 { -1.0 } else { 1.0 };
    sign * ca.dot(ca).min(cb.dot(cb)).sqrt()
}

/// Material y coordenadas de textura del punto de impacto.
struct SurfaceInfo {
    material: MaterialId,
    uv: Vec2,
    tangent_u: Vec3,
    tangent_v: Vec3,
    region: Region,
}

impl SurfaceInfo {
    fn plain(material: MaterialId, uv: Vec2, tangent_u: Vec3, tangent_v: Vec3) -> Self {
        Self {
            material,
            uv,
            tangent_u,
            tangent_v,
            region: Region::Plain,
        }
    }
}

/// Proyeccion de caja en un marco `(x, y, z)` dado por sus ejes en el marco
/// local: la cara dominante decide que par de ejes es el uv.
fn boxed(q: Vec3, n: Vec3, axes: [Vec3; 3], offset: Vec2, material: MaterialId) -> SurfaceInfo {
    let nq = Vec3::new(n.dot(axes[0]), n.dot(axes[1]), n.dot(axes[2])).abs();
    let (uv, tu, tv) = if nq.x >= nq.y && nq.x >= nq.z {
        (Vec2::new(q.z, q.y), axes[2], axes[1])
    } else if nq.y >= nq.z {
        (Vec2::new(q.x, q.z), axes[0], axes[2])
    } else {
        (Vec2::new(q.x, q.y), axes[0], axes[1])
    };
    SurfaceInfo::plain(material, uv + offset, tu, tv)
}

/// Ejes del marco de un elemento radial a angulo `theta`, en el marco local.
fn radial_axes(theta: f32) -> [Vec3; 3] {
    let (s, c) = theta.sin_cos();
    [Vec3::new(-s, 0.0, c), Vec3::Y, Vec3::new(c, 0.0, s)]
}

fn surface(p: Vec3, n: Vec3, glass: bool) -> SurfaceInfo {
    let (_, part) = evaluate(p, glass);
    let module = |k: i32| {
        let axes = radial_axes(k as f32 * TAU / MODULE_COUNT as f32);
        (module_frame(p, k), axes, Vec2::new(k as f32 * 1.73, k as f32 * 0.91))
    };
    match part {
        Part::Body(k) if k.rem_euclid(MODULE_COUNT) % 3 == 0 => {
            // Habitat: las caras axiales llevan ventanas iluminadas. Sin
            // desplazamiento de uv para que la textura sepa donde estan.
            let (q, axes, offset) = module(k);
            let axial = n.y.abs() > n.dot(axes[0]).abs() && n.y.abs() > n.dot(axes[2]).abs();
            let mut info = boxed(q, n, axes, if axial { Vec2::ZERO } else { offset }, MaterialId::Hull);
            if axial {
                info.region = Region::Windows;
            }
            info
        }
        Part::Body(k) | Part::Mast(k) | Part::Equipment(k) | Part::Pipe(k) => {
            let (q, axes, offset) = module(k);
            boxed(q, n, axes, offset, MaterialId::Hull)
        }
        Part::Band(k) => {
            let (q, axes, offset) = module(k);
            boxed(q, n, axes, offset, MaterialId::Gold)
        }
        Part::Tank(k) => {
            let (q, axes, _) = module(k);
            let around = (q.z - 0.122).atan2(q.y.abs() - 0.035) * TANK_RADIUS;
            SurfaceInfo::plain(MaterialId::Hull, Vec2::new(q.x, around), axes[0], axes[2])
        }
        Part::Panel(k) => {
            let (q, axes, _) = module(k);
            SurfaceInfo::plain(
                MaterialId::Solar,
                Vec2::new(q.x + PANEL_HALF.x, q.z - PANEL_CENTER + PANEL_HALF.z),
                axes[0],
                axes[2],
            )
        }
        Part::Nozzle(k) => {
            let (q, axes, _) = module(k);
            let local = Vec3::new(q.x.abs() - NOZZLE_OFFSET, q.y - NOZZLE_CENTER, q.z);
            let rho = Vec2::new(local.x, local.z).length();
            let exhaust = n.y < -0.7 && local.y < -NOZZLE_HALF + 0.01;
            SurfaceInfo {
                material: MaterialId::Nozzle,
                uv: if exhaust {
                    Vec2::new(rho / NOZZLE_EXIT, 1.0)
                } else {
                    Vec2::new(
                        local.z.atan2(local.x) * 0.06,
                        (NOZZLE_HALF - local.y) / (2.0 * NOZZLE_HALF),
                    )
                },
                tangent_u: axes[0],
                tangent_v: axes[2],
                region: if exhaust { Region::Exhaust } else { Region::Plain },
            }
        }
        Part::Connector(j) | Part::Flange(j) => {
            let theta = (j as f32 + 0.5) * TAU / MODULE_COUNT as f32;
            let axes = radial_axes(theta);
            let q = frame(p, theta) - Vec3::new(0.0, 0.0, RING_RADIUS);
            let around = q.z.atan2(q.y) * CONNECTOR_RADIUS;
            SurfaceInfo::plain(MaterialId::Hull, Vec2::new(q.x + j as f32, around), axes[0], Vec3::Y)
        }
        Part::Spoke(k) => {
            let theta = k as f32 * TAU / SPOKE_COUNT as f32;
            let axes = radial_axes(theta);
            let q = frame(p, theta);
            let around = q.x.atan2(q.y.abs() - 0.02) * SPOKE_RADIUS;
            SurfaceInfo::plain(MaterialId::Hull, Vec2::new(q.z, around), axes[2], axes[0])
        }
        Part::Hub | Part::HubFlange | Part::Collar => {
            let material = if part == Part::Hub {
                MaterialId::Ceramic
            } else {
                MaterialId::Hull
            };
            let angle = p.z.atan2(p.x);
            let (s, c) = angle.sin_cos();
            if n.y.abs() > 0.6 {
                let rho = Vec2::new(p.x, p.z).length();
                let instruments = part == Part::Hub && n.y < 0.0 && rho < DOME_RADIUS;
                let mut info = SurfaceInfo::plain(material, Vec2::new(p.x, p.z), Vec3::X, Vec3::Z);
                if instruments {
                    info.region = Region::Instruments;
                }
                info
            } else {
                SurfaceInfo::plain(material, Vec2::new(angle * HUB_RADIUS, p.y), Vec3::new(-s, 0.0, c), Vec3::Y)
            }
        }
        Part::Ranger => {
            // Panza de losetas negras y lomo blanco, como un transbordador.
            let material = if n.z < -0.25 {
                MaterialId::Ceramic
            } else {
                MaterialId::Hull
            };
            boxed(p, n, [Vec3::X, Vec3::Y, Vec3::Z], Vec2::new(0.37, 0.0), material)
        }
        Part::Dome => {
            let d = p - Vec3::new(0.0, -HUB_HALF, 0.0);
            let azimuth = d.z.atan2(d.x);
            let (s, c) = azimuth.sin_cos();
            let length = d.length().max(1e-6);
            let elevation = (-d.y / length).clamp(-1.0, 1.0).asin();
            SurfaceInfo::plain(
                MaterialId::Glass,
                Vec2::new(azimuth + PI, elevation),
                Vec3::new(-s, 0.0, c),
                (-Vec3::Y - d * (-d.y / (length * length))).normalize_or_zero(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_lighting() -> Lighting {
        // Un piso luminoso: suficiente para probar el sombreado.
        let count = 1024;
        let samples: Vec<Vec3> = (0..count)
            .map(|i| {
                if crate::scene::lighting::fibonacci(i, count).y < 0.0 {
                    Vec3::splat(0.3)
                } else {
                    Vec3::ZERO
                }
            })
            .collect();
        Lighting::from_samples(&samples)
    }

    #[test]
    fn geometry_is_closed_inside_its_bounding_sphere() {
        // Centro de un modulo: adentro. Superficie de la esfera envolvente:
        // afuera de toda pieza.
        assert!(evaluate(Vec3::new(RING_RADIUS, 0.0, 0.0), true).0 < 0.0);
        for i in 0..400 {
            let a = i as f32 * 2.399_963;
            let y = 1.0 - 2.0 * (i as f32 + 0.5) / 400.0;
            let r = (1.0 - y * y).sqrt();
            let p = Vec3::new(r * a.cos(), y, r * a.sin()) * BOUND;
            assert!(evaluate(p, true).0 > 0.0, "pieza fuera de la envolvente en {p:?}");
        }
        // Los doce modulos existen, no solo los evaluados por la repeticion.
        for k in 0..MODULE_COUNT {
            let theta = k as f32 * TAU / MODULE_COUNT as f32;
            let p = Vec3::new(theta.cos(), 0.0, theta.sin()) * RING_RADIUS;
            match evaluate(p, true).1 {
                Part::Body(j) => assert_eq!(j.rem_euclid(MODULE_COUNT), k),
                other => panic!("modulo {k} sin cuerpo: {other:?}"),
            }
        }
        // El Ranger, los tuneles y los radios tambien estan.
        assert_eq!(evaluate(Vec3::new(0.0, 0.38, 0.0), true).1, Part::Ranger);
        let between = TAU / MODULE_COUNT as f32 * 0.5;
        let p = Vec3::new(between.cos(), 0.0, between.sin()) * RING_RADIUS;
        assert!(matches!(evaluate(p, true).1, Part::Connector(_) | Part::Flange(_)));
        assert!(matches!(evaluate(Vec3::new(0.55, 0.02, 0.0), true).1, Part::Spoke(_)));
    }

    #[test]
    fn region_bounds_never_overestimate_the_distance() {
        // La distancia con cotas no puede superar la evaluacion completa de
        // todas las piezas, o la marcha atravesaria el casco.
        for i in 0..3000 {
            let a = i as f32 * 2.399_963;
            let y = (i as f32 * 0.618_034).fract() * 1.4 - 0.7;
            let r = (i as f32 * 0.414_213).fract() * 1.7;
            let p = Vec3::new(r * a.cos(), y, r * a.sin());
            let fast = evaluate(p, true).0;
            let mut exact = (f32::INFINITY, Part::Hub);
            let angle = p.z.atan2(p.x);
            ring(p, angle, &mut exact);
            spokes(p, angle, &mut exact);
            core(p, true, &mut exact);
            assert!(fast <= exact.0 + 1e-5, "{p:?}: {fast} > {}", exact.0);
        }
    }

    #[test]
    fn rays_hit_the_ship_and_see_its_materials() {
        let ship = Ship::new(0.0, test_lighting());
        let noise = NoiseTable::new(config::NOISE_SEED);
        let axis = ship.rotation * -Vec3::Y;
        let far = ship.center() + axis * (40.0 * config::SHIP_SCALE);
        let t = ship.intersect(far, ship.center()).expect("el rayo axial debe pegar");
        let hit = far + (ship.center() - far) * t;
        // Desde atras, el eje de la nave atraviesa la cupula primero.
        let p = ship.to_local(hit);
        assert_eq!(evaluate(p, true).1, Part::Dome);
        let view = (ship.center() - far).normalize();
        let shading = ship.shade(&noise, hit, view, true, 1.0e-6);
        assert!(shading.refract_weight.x > 0.5, "la cupula deja pasar la luz");
        assert!(shading.normal.dot(view) < 0.0);
        assert!(shading.color.min_element() >= 0.0 && shading.color.x.is_finite());
        let miss = ship.center() + Vec3::new(0.0, 5.0, 0.0);
        assert!(ship.intersect(miss, miss + Vec3::X).is_none());
    }

    #[test]
    fn light_inside_the_dome_refracts_and_reaches_the_instruments() {
        let ship = Ship::new(0.0, test_lighting());
        let entry_local = Vec3::new(0.05, -HUB_HALF - (DOME_RADIUS * DOME_RADIUS - 0.0025).sqrt(), 0.0);
        let inward = ship.rotation * Vec3::new(0.1, 1.0, 0.05).normalize();
        match ship.through_glass(ship.to_world(entry_local), inward) {
            GlassExit::Floor { absorption, .. } => {
                assert!(absorption.max_element() < 1.0 && absorption.min_element() > 0.0);
            }
            _ => panic!("un rayo hacia el nucleo debe llegar al tablero"),
        }
    }

    #[test]
    fn the_ship_flies_just_above_the_gas() {
        let center = ship_center();
        let lowest = center.y - BOUND * config::SHIP_SCALE;
        assert!(lowest > 0.0 && center.y < 1.0, "la nave roza el gas sin tocar el plano");
        assert!(ship_forward().dot(-center) > 0.0, "vuela hacia el agujero");
    }
}
