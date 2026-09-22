//! Relatividad general: geodesicas nulas de Schwarzschild y corrimiento de la luz.
//!
//! Trayectorias y corrimiento para Schwarzschild. El perfil termico al final
//! es una aproximacion de disco delgado, separada de las ecuaciones geodesicas.
//!
//! # Unidades
//!
//! Todo el proyecto trabaja con `rs = 2GM/c^2 = 1`, o sea que la masa vale
//! `M = 1/2` y las velocidades estan en unidades de `c`. Con esa eleccion los
//! radios caracteristicos son numeros fijos y reconocibles:
//!
//! | Radio | Valor | Que es |
//! | --- | --- | --- |
//! | `rs` | 1.0 | Horizonte de eventos |
//! | `3M` | 1.5 | Esfera de fotones: la luz puede orbitar |
//! | `3sqrt(3)M` | 2.598 | Radio aparente de la sombra, visto de lejos |
//! | `6M` | 3.0 | ISCO: la orbita circular estable mas cercana |
//!
//! # La ecuacion de la geodesica
//!
//! Para una geodesica nula en Schwarzschild el movimiento es plano, porque el
//! momento angular `L = r x v` se conserva en modulo y en direccion. En ese
//! plano, con `u = 1/r`, la trayectoria cumple exactamente
//!
//! ```text
//!   d^2u/dphi^2 + u = 3 M u^2
//! ```
//!
//! El termino de la izquierda solo es la orbita newtoniana; el `3 M u^2` de la
//! derecha es toda la relatividad general que hay en este problema. Es lo que
//! hace que exista una esfera de fotones y que la sombra se vea mas grande que
//! el horizonte.
//!
//! Pasando esa ecuacion a una aceleracion central con la ecuacion de Binet, y de
//! ahi a vectores, queda
//!
//! ```text
//!   a = -(3/2) rs h^2 r / |r|^5,     h^2 = |r x v|^2
//! ```
//!
//! que es lo que integra `geodesic_acceleration`. `h^2` se conserva, asi que se
//! calcula una sola vez por rayo y despues la aceleracion es una formula cerrada.
//!
//! La diferencia con la gravedad newtoniana es la potencia: Newton da `1/r^2` y
//! esto da `1/r^4`. Por eso la deflexion cae mucho mas rapido con la distancia y,
//! cerca del agujero, es mucho mas violenta.

use glam::Vec3;

use crate::config;

/// Evita divisiones por cero en los factores metricos justo sobre el horizonte.
const EPSILON: f32 = 1.0e-6;

/// Constantes de un rayo que hacen falta para calcular el corrimiento.
///
/// Las dos se fijan al salir de la camara y no cambian en toda la geodesica, asi
/// que se calculan una vez por pixel y se pasan a cada muestra del gas.
#[derive(Clone, Copy, Debug)]
pub struct RayFrame {
    /// `L_z/E` del foton fisico: el parametro de impacto proyectado sobre el eje
    /// del disco, con signo. Es lo que distingue el lado que se acerca del que se
    /// aleja.
    pub lz_over_e: f32,
    /// Radio del observador estatico. Fija su propio corrimiento gravitacional.
    pub observer_radius: f32,
}

/// Momento angular especifico del foton, `L = r x v`.
///
/// Se conserva a lo largo de toda la geodesica, en modulo y en direccion. Que se
/// conserve la direccion es lo que hace que la trayectoria sea plana.
#[inline]
pub fn angular_momentum(pos: Vec3, vel: Vec3) -> Vec3 {
    pos.cross(vel)
}

/// Aceleracion de la geodesica nula: `a = -(3/2) rs h^2 r / |r|^5`.
///
/// `h_sq` es `|r x v|^2`, constante a lo largo del rayo.
#[inline]
pub fn geodesic_acceleration(pos: Vec3, h_sq: f32) -> Vec3 {
    let r_sq = pos.length_squared().max(EPSILON);
    // |r|^5 = (r^2)^2 * |r|
    let r_fifth = r_sq * r_sq * r_sq.sqrt();
    pos * (-1.5 * config::SCHWARZSCHILD_RADIUS * h_sq / r_fifth)
}

/// Energia conservada del foton, `E = -p_t`.
///
/// Sale de imponer la condicion nula `g_uv p^u p^v = 0` sobre la parametrizacion
/// que usa el integrador:
///
/// ```text
///   E^2 = |v|^2 - rs h^2 / r^3
/// ```
///
/// Igual que `h`, se conserva: sirve como control de que la integracion no se
/// esta yendo, y hace falta para el cociente `L/E` del efecto Doppler.
#[inline]
pub fn photon_energy(pos: Vec3, vel: Vec3, h_sq: f32) -> f32 {
    let r = pos.length().max(EPSILON);
    (vel.length_squared() - config::SCHWARZSCHILD_RADIUS * h_sq / (r * r * r))
        .max(EPSILON)
        .sqrt()
}

/// Velocidad angular de una orbita circular ecuatorial, en coordenada `t`.
///
/// `Omega = sqrt(M / r^3)`. Es identica a la tercera ley de Kepler newtoniana, y
/// eso no es casualidad ni aproximacion: en coordenadas de Schwarzschild el
/// resultado es exactamente el mismo. Lo relativista aparece en otro lado, en el
/// factor `u^t` con que el gas mide su propio tiempo.
#[inline]
pub fn keplerian_omega(radius: f32) -> f32 {
    (config::BLACK_HOLE_MASS / (radius * radius * radius)).sqrt()
}

/// Factor de corrimiento total `g = nu_observada / nu_emitida` para gas en orbita
/// circular visto por un observador estatico.
///
/// ```text
///   g = sqrt(1 - 3M/r_emit) / [ (1 - Omega L_z/E) sqrt(1 - rs/r_obs) ]
/// ```
///
/// Los tres factores son tres efectos distintos, y conviene distinguirlos:
///
/// - **`sqrt(1 - 3M/r_emit)`** es el inverso de `u^t`, la dilatacion temporal del
///   gas. Mezcla el corrimiento gravitacional (por estar hondo en el pozo) con la
///   dilatacion por su propia velocidad orbital. Se anula en `r = 3M`, la esfera
///   de fotones: ahi una orbita circular exigiria ir a la velocidad de la luz.
/// - **`1 - Omega L_z/E`** es el Doppler longitudinal. `L_z/E` es el parametro de
///   impacto proyectado sobre el eje del disco, con signo: cambia de lado segun
///   si el gas viene hacia el observador o se aleja. Es el termino que hace que
///   un lado del disco sea mucho mas brillante que el otro.
/// - **`sqrt(1 - rs/r_obs)`** es el corrimiento del observador. Un observador
///   estatico hondo en el pozo ve todo lo de afuera corrido al azul.
///
/// Con `g` se obtienen las dos cosas que hacen falta de un golpe: la temperatura
/// observada es `g T`, y la intensidad bolometrica observada es `g^4` veces la
/// emitida. El invariante espectral es `I_nu/nu^3`; integrar en frecuencia
/// agrega el cuarto factor de g. Como un cuerpo negro a `g T` emite
/// `sigma (g T)^4`, calcular el color y el brillo a partir de `g T` ya incluye
/// el `g^4` sin tener que aplicarlo aparte.
#[inline]
pub fn redshift_factor(radius_emit: f32, radius_obs: f32, lz_over_e: f32) -> f32 {
    let omega = keplerian_omega(radius_emit);

    let emitter_time = (1.0 - 3.0 * config::BLACK_HOLE_MASS / radius_emit).max(EPSILON);
    let observer_time = (1.0 - config::SCHWARZSCHILD_RADIUS / radius_obs).max(EPSILON);
    let doppler = 1.0 - omega * lz_over_e;

    if doppler.abs() < EPSILON {
        return 0.0;
    }

    emitter_time.sqrt() / (doppler * observer_time.sqrt())
}

/// Corrimiento al azul que ve un observador estatico para luz que viene del
/// infinito: `1 / sqrt(1 - rs/r_obs)`.
///
/// Es lo que le pasa al cielo de fondo. Lejos es despreciable, pero acercandose
/// al horizonte crece sin cota: a `1.2 rs` vale 2.4, o sea que las estrellas se
/// ven al doble de temperatura de color y unas treinta veces mas brillantes.
#[inline]
pub fn blueshift_from_infinity(radius_obs: f32) -> f32 {
    let observer = (1.0 - config::SCHWARZSCHILD_RADIUS / radius_obs).max(EPSILON);
    1.0 / observer.sqrt()
}

/// Perfil termico newtoniano de disco delgado con torque nulo en la ISCO.
/// No incluye las correcciones relativistas del perfil de Novikov-Thorne.
///
/// ```text
///   T(r) proporcional a (r/r_in)^(-3/4) * [1 - sqrt(r_in/r)]^(1/4)
/// ```
///
/// El `r^(-3/4)` es el perfil clasico de Shakura-Sunyaev: el gas de adentro
/// disipa mas energia por unidad de area. El corchete es la condicion de borde
/// de torque nulo en el borde interno, y tiene una consecuencia visible: la
/// temperatura **no** es maxima en la ISCO sino en `r = (49/36) r_in`, un poco
/// mas afuera, y cae a cero justo en el borde. O sea que el anillo mas brillante
/// del disco no esta pegado al agujero, hay un hueco oscuro entre los dos.
///
/// El resultado esta normalizado para que su maximo valga `DISK_TEMPERATURE_PEAK`.
#[inline]
pub fn disk_temperature(radius: f32) -> f32 {
    if radius <= config::DISK_INNER_RADIUS {
        return 0.0;
    }
    let x = radius / config::DISK_INNER_RADIUS;
    let profile = x.powf(-0.75) * (1.0 - x.powf(-0.5)).max(0.0).powf(0.25);
    config::DISK_TEMPERATURE_PEAK * profile / config::THIN_DISK_PROFILE_PEAK
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn doppler_matches_local_special_relativity() {
        let radius: f32 = 6.0;
        let observer: f32 = 1000.0;
        let beta = (config::BLACK_HOLE_MASS / (radius - config::SCHWARZSCHILD_RADIUS)).sqrt();
        let gamma = 1.0 / (1.0 - beta * beta).sqrt();
        let b = radius / (1.0 - config::SCHWARZSCHILD_RADIUS / radius).sqrt();
        let gravity = ((1.0 - config::SCHWARZSCHILD_RADIUS / radius)
            / (1.0 - config::SCHWARZSCHILD_RADIUS / observer))
            .sqrt();
        let approaching = redshift_factor(radius, observer, b);
        let receding = redshift_factor(radius, observer, -b);
        assert!((approaching - gravity / (gamma * (1.0 - beta))).abs() < 1e-5);
        assert!((receding - gravity / (gamma * (1.0 + beta))).abs() < 1e-5);
        assert!(approaching > 1.0 && receding < 1.0);
        assert!(approaching.powi(4) > 8.0 * receding.powi(4));
        let blue = crate::math::blackbody::planckian_rgb(7000.0 * approaching);
        let red = crate::math::blackbody::planckian_rgb(7000.0 * receding);
        assert!(blue.z / blue.x > red.z / red.x);
    }
    #[test]
    fn thermal_profile_has_zero_torque_inner_edge() {
        assert_eq!(disk_temperature(config::DISK_INNER_RADIUS), 0.0);
        let peak = disk_temperature(config::DISK_INNER_RADIUS * 49.0 / 36.0);
        assert!((peak / config::DISK_TEMPERATURE_PEAK - 1.0).abs() < 1e-4);
    }
}
