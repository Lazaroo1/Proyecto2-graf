//! Cielo de fondo: estrellas y brillo difuso de la galaxia.
//!
//! # Por que hay un fondo
//!
//! Sin el, la lente gravitacional es casi invisible. La sombra sobre negro se ve
//! igual que un disco negro cualquiera. Con estrellas detras, en cambio, se ve lo
//! que realmente hace la gravedad: las estrellas que pasan cerca del agujero se
//! arrastran, se estiran en arcos, y las que estan justo detras aparecen
//! duplicadas a los costados o cerradas en un anillo de Einstein. Al mover la
//! camara todo el campo se deforma alrededor del agujero. Eso es lo que hace
//! legible el efecto.
//!
//! # Como se generan
//!
//! La direccion se proyecta sobre la cara dominante de un cubo y se cuadricula en
//! ese plano 2D. Un hash de la celda decide si hay estrella, donde cae dentro de
//! la celda, que temperatura tiene y cuanto brilla. Es procedural y sin memoria:
//! dos rayos que miran al mismo lado siempre ven la misma estrella.
//!
//! La parametrizacion es 2D y no 3D, y la diferencia importa. Cuadricular el
//! espacio en cubos y poner la estrella en un punto del cubo parece mas simple,
//! pero ese punto casi nunca cae exactamente sobre la esfera de direcciones: la
//! celda que se consulta al mirar hacia la estrella termina siendo otra que la
//! que la genero, y la estrella aparece solo en el pedazo donde las dos
//! coinciden. El resultado son rayas, no puntos. Con una cuadricula 2D sobre la
//! cara del cubo, la celda de una direccion y la celda de la estrella son la
//! misma por construccion.
//!
//! La posicion dentro de la celda se confina lejos del borde. Solo se consulta la
//! celda que contiene la direccion, no las vecinas, asi que una estrella pegada al
//! borde se veria cortada por la mitad.
//!
//! # Corrimiento al azul
//!
//! Un observador estatico hondo en el pozo gravitatorio ve toda la luz que viene
//! de lejos corrida al azul, por un factor `1/sqrt(1 - rs/r_obs)`. A 25 radios de
//! Schwarzschild son apenas un 2%, pero acercandose al horizonte se vuelve
//! enorme: el cielo entero se pone mas azul y mas brillante. Es el mismo factor
//! que aparece en el corrimiento del disco, y aca esta por la misma razon.

use glam::{Mat3, Vec2, Vec3};

use crate::config;
use crate::math::blackbody;
use crate::math::curves;
use crate::math::noise::NoiseTable;
use crate::scene::relativity;

/// Transformacion compartida por frame. Se aplica a la direccion de ESCAPE
/// de cada geodesica, nunca a las coordenadas de pantalla: asi las estrellas
/// cruzan la lente formando arcos y duplicaciones en el lugar correcto.
pub struct SkyFrame {
    rotation: Mat3,
    blueshift: f32,
}

impl SkyFrame {
    pub fn new(time: f32, observer_radius: f32) -> Self {
        let angle = (time * config::SKY_DRIFT_SPEED).rem_euclid(std::f32::consts::TAU);
        Self {
            rotation: Mat3::from_rotation_z(0.23)
                * Mat3::from_rotation_y(angle)
                * Mat3::from_rotation_z(-0.23),
            blueshift: relativity::blueshift_from_infinity(observer_radius),
        }
    }

    pub fn sample(&self, escape_direction: Vec3) -> Vec3 {
        if escape_direction == Vec3::ZERO {
            return Vec3::ZERO;
        }
        let direction = self.rotation * escape_direction;
        let mut color = Vec3::ZERO;
        // Dos escalas: puntos pequenos y unas pocas estrellas que sirven de
        // referencias para seguir el estiramiento. Nada gira en torno a la
        // sombra a mano; ese movimiento sale del mapa de rayos.
        for (scale, density, size, brightness, seed) in [
            (
                config::PREVIEW_STAR_GRID,
                config::PREVIEW_STAR_DENSITY,
                config::PREVIEW_STAR_SIZE,
                config::PREVIEW_STAR_BRIGHTNESS,
                0,
            ),
            (72.0, 0.10, 0.075, 0.45, 917),
        ] {
            if let Some(star) = star_layer(direction, scale, density, size, brightness, seed) {
                color +=
                    blackbody::planckian_rgb(star.temperature * self.blueshift) * star.brightness;
            }
        }
        color * self.blueshift.powi(4)
    }
}

/// Color del cielo en una direccion, visto por un observador estatico a
/// `observer_radius`.
pub fn background(noise: &NoiseTable, direction: Vec3, observer_radius: f32) -> Vec3 {
    let blueshift = relativity::blueshift_from_infinity(observer_radius);

    let mut color = nebula(noise, direction);

    if let Some(star) = star(direction) {
        let observed_temperature = star.temperature * blueshift;
        // El brillo de la estrella es intrinseco y va aparte de la temperatura:
        // a diferencia del disco, lo que llega de una estrella depende tambien de
        // su tamano y su distancia, no solo de lo caliente que este.
        color += blackbody::planckian_rgb(observed_temperature) * star.brightness;
    }

    // El `^4` transporta intensidad bolometrica; el invariante es I_nu/nu^3.
    // Va una sola vez y sobre todo el cielo, estrellas y fondo difuso por igual.
    color * blueshift.powi(4)
}

/// Una estrella encontrada en la celda que mira el rayo.
struct Star {
    /// Temperatura efectiva, en kelvin.
    temperature: f32,
    /// Brillo ya atenuado por la distancia al centro de la estrella.
    brightness: f32,
}

/// Busca una estrella en la direccion dada.
fn star(direction: Vec3) -> Option<Star> {
    star_layer(
        direction,
        config::STAR_GRID,
        config::STAR_DENSITY,
        config::STAR_SIZE,
        config::STAR_BRIGHTNESS,
        0,
    )
}

fn star_layer(
    direction: Vec3,
    grid: f32,
    density: f32,
    size: f32,
    brightness_scale: f32,
    seed: u32,
) -> Option<Star> {
    let (face, u, v) = cube_face(direction);

    // `u` y `v` viven en [-1, 1] sobre la cara, o sea 90 grados de arco. La
    // media escala convierte ese rango de 2 unidades en `STAR_GRID` celdas.
    let p = Vec2::new(u, v) * (grid * 0.5);
    let cell = p.floor();

    let hash = hash_cell(cell, face.wrapping_add(seed));
    if to_unit(hash) > density {
        return None;
    }

    // Posicion dentro de la celda, confinada a la mitad central para que la
    // estrella no quede cortada por el borde.
    let center = cell
        + Vec2::new(
            0.3 + 0.4 * to_unit(hash.rotate_left(5)),
            0.3 + 0.4 * to_unit(hash.rotate_left(13)),
        );

    let offset = (p - center).length() / size;
    let falloff = (-offset * offset).exp();
    if falloff < 1.0e-3 {
        return None;
    }

    // Distribucion de brillo sesgada a estrellas debiles: el cubo de una uniforme
    // deja muchas apenas visibles y unas pocas notorias, que es como se ve un
    // cielo real.
    let magnitude = to_unit(hash.rotate_left(9));
    let brightness = magnitude * magnitude * magnitude * brightness_scale * falloff;

    let temperature = curves::remap(
        to_unit(hash.rotate_left(17)),
        0.0,
        1.0,
        config::STAR_TEMPERATURE_MIN,
        config::STAR_TEMPERATURE_MAX,
    );

    Some(Star {
        temperature,
        brightness,
    })
}

/// Proyecta una direccion sobre la cara dominante de un cubo.
///
/// Devuelve el indice de cara y las dos coordenadas en `[-1, 1]`. Las celdas
/// quedan algo mas chicas en angulo hacia las esquinas de cada cara, que es la
/// distorsion habitual de un mapa de cubo y no se nota en un campo de estrellas.
fn cube_face(direction: Vec3) -> (u32, f32, f32) {
    let a = direction.abs();
    if a.x >= a.y && a.x >= a.z {
        let face = if direction.x > 0.0 { 0 } else { 1 };
        (face, direction.y / a.x, direction.z / a.x)
    } else if a.y >= a.z {
        let face = if direction.y > 0.0 { 2 } else { 3 };
        (face, direction.x / a.y, direction.z / a.y)
    } else {
        let face = if direction.z > 0.0 { 4 } else { 5 };
        (face, direction.x / a.z, direction.y / a.z)
    }
}

/// Brillo difuso de fondo: el resplandor de las estrellas que no se resuelven.
///
/// Tres octavas de fBm sobre la direccion, tenidas de un azul frio. No pretende
/// ser un mapa del cielo real; esta para que el fondo no sea negro absoluto, que
/// es lo unico que el espacio profundo tampoco es.
fn nebula(noise: &NoiseTable, direction: Vec3) -> Vec3 {
    let density = noise.fbm(direction * 2.5, Vec3::ZERO, 3);
    let shaped = curves::smoothstep(0.45, 1.0, density);
    Vec3::new(0.35, 0.42, 0.75) * shaped * config::NEBULA_BRIGHTNESS
}

/// Hash de una celda del cielo.
#[inline]
fn hash_cell(cell: Vec2, face: u32) -> u32 {
    let mut h = (cell.x as i32 as u32).wrapping_mul(0x9E37_79B9)
        ^ (cell.y as i32 as u32).wrapping_mul(0x85EB_CA6B)
        ^ face.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    h
}

/// Interpreta un `u32` como una fraccion en `[0, 1)`.
#[inline]
fn to_unit(bits: u32) -> f32 {
    (bits >> 8) as f32 / 16_777_216.0
}
