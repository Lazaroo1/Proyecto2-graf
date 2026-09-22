//! El foton: estado de una geodesica nula y su integracion.
//!
//! La fisica esta en [`crate::scene::relativity`]; aca esta el integrador y los
//! criterios de terminacion.
//!
//! # Se traza al reves
//!
//! El rayo sale de la camara y va hacia atras en el tiempo, buscando de donde
//! vino la luz que llega a ese pixel. Eso no cambia nada de la geometria: una
//! geodesica recorrida al reves sigue siendo la misma geodesica. Lo unico que
//! hay que cuidar es el signo del momento angular cuando se calcula el Doppler,
//! porque ahi si importa hacia donde viajaba el foton de verdad.
//!
//! # Integrador
//!
//! Runge-Kutta de cuarto orden. Cuesta cuatro evaluaciones de la aceleracion por
//! paso, pero la aceleracion son diez operaciones y una raiz, mientras que
//! muestrear el gas cuesta varias evaluaciones de ruido. Conviene mucho mas dar
//! pasos largos y precisos que pasos cortos y baratos: se gasta menos en el gas,
//! que es lo caro, y ademas el anillo de fotones sale bien.
//!
//! Con Euler, que era lo que habia antes, el error se acumula justo donde mas se
//! nota: los rayos que casi orbitan son los que forman el anillo, y son los que
//! mas vueltas dan cerca del agujero.

use glam::Vec3;

use crate::config;
use crate::math::sdf;
use crate::scene::relativity;

/// Estado de un foton a lo largo de su geodesica.
pub struct Photon {
    /// Posicion actual, en coordenadas de Schwarzschild leidas como cartesianas.
    pub pos: Vec3,
    /// Derivada de la posicion respecto del parametro afin.
    ///
    /// Ojo: su modulo **no** es constante ni vale 1. La condicion nula relaciona
    /// `|v|` con el radio, y es justamente esa variacion la que codifica que la
    /// parametrizacion afin difiere de una distancia espacial euclidiana.
    pub vel: Vec3,
    /// `|r x v|^2`, conservado. La aceleracion solo depende de esto y de `r`.
    h_sq: f32,
    /// `L_z/E` del foton fisico, conservado. Es lo que entra al efecto Doppler.
    pub lz_over_e: f32,
}

impl Photon {
    /// Arranca un foton desde la camara, hacia atras en el tiempo.
    pub fn from_camera(origin: Vec3, direction: Vec3) -> Self {
        // La camara mide angulos en un marco local ortonormal, no en las
        // coordenadas de Schwarzschild. Con energia local = 1, dr/dlambda
        // lleva sqrt(1-rs/r), mientras que la componente tangencial no.
        let local = direction.normalize_or_zero();
        let radial = origin.normalize();
        let lapse = (1.0 - config::SCHWARZSCHILD_RADIUS / origin.length()).sqrt();
        let radial_component = radial * local.dot(radial);
        let vel = local - radial_component + radial_component * lapse;
        let momentum = relativity::angular_momentum(origin, vel);
        let h_sq = momentum.length_squared();
        let energy = relativity::photon_energy(origin, vel, h_sq);

        // El foton real viaja en sentido contrario al rayo que trazamos, asi que
        // su momento angular es el opuesto. El signo decide que lado del disco se
        // ve corrido al azul: invertirlo por error daria el efecto espejado.
        let lz_over_e = -momentum.y / energy;

        Self {
            pos: origin,
            vel,
            h_sq,
            lz_over_e,
        }
    }

    /// Avanza un paso del parametro afin con Runge-Kutta de cuarto orden.
    pub fn advance(&mut self, dt: f32) {
        let h = self.h_sq;
        let (p0, v0) = (self.pos, self.vel);

        let a0 = relativity::geodesic_acceleration(p0, h);
        let (p1, v1) = (p0 + v0 * (dt * 0.5), v0 + a0 * (dt * 0.5));

        let a1 = relativity::geodesic_acceleration(p1, h);
        let (p2, v2) = (p0 + v1 * (dt * 0.5), v0 + a1 * (dt * 0.5));

        let a2 = relativity::geodesic_acceleration(p2, h);
        let (p3, v3) = (p0 + v2 * dt, v0 + a2 * dt);

        let a3 = relativity::geodesic_acceleration(p3, h);

        self.pos = p0 + (v0 + 2.0 * v1 + 2.0 * v2 + v3) * (dt / 6.0);
        self.vel = v0 + (a0 + 2.0 * a1 + 2.0 * a2 + a3) * (dt / 6.0);
    }

    /// Paso afin limitado por curvatura y por una fraccion del radio.
    /// El muestreo del volumen agrega su propio limite en raymarch::volume_step.
    pub fn step_length(&self) -> f32 {
        let acceleration = relativity::geodesic_acceleration(self.pos, self.h_sq).length();
        let curvature = if acceleration > 1.0e-9 {
            config::STEP_ANGLE_TOLERANCE * self.vel.length() / acceleration
        } else {
            config::MAX_STEP
        };
        let radial = self.pos.length() * config::STEP_RADIUS_FRACTION;

        curvature
            .min(radial)
            .clamp(config::MIN_STEP, config::MAX_STEP)
    }

    /// Si el foton ya no puede volver a salir.
    ///
    /// Dos casos. El obvio es haber cruzado el horizonte. El otro es mas util:
    /// **adentro de la esfera de fotones y yendo hacia adentro**. En Schwarzschild
    /// el potencial efectivo de una geodesica nula tiene su unico maximo en `3M`,
    /// asi que una vez adentro y cayendo no existe punto de retorno posible. El
    /// resultado es exacto, no una heuristica, y ahorra las decenas de pasos que
    /// costaria seguir la espiral hasta el horizonte.
    pub fn captured(&self) -> bool {
        if sdf::sd_sphere(self.pos, config::SCHWARZSCHILD_RADIUS) < 0.0 {
            return true;
        }
        self.pos.length() < config::PHOTON_SPHERE_RADIUS && self.vel.dot(self.pos) < 0.0
    }

    /// Si el foton ya se fue lo bastante lejos como para no volver.
    pub fn escaped(&self) -> bool {
        self.pos.length() > config::ESCAPE_RADIUS && self.vel.dot(self.pos) > 0.0
    }

    /// Direccion asintotica del rayo, para muestrear el cielo de fondo.
    pub fn direction(&self) -> Vec3 {
        self.vel.normalize_or_zero()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_camera_angles_give_the_correct_impact_parameter() {
        for radius in [2.2_f32, 6.0, 24.0, 120.0] {
            let angle: f32 = 0.31;
            let photon =
                Photon::from_camera(Vec3::Z * radius, Vec3::new(angle.sin(), 0.0, -angle.cos()));
            let h = relativity::angular_momentum(photon.pos, photon.vel).length();
            let energy = relativity::photon_energy(photon.pos, photon.vel, h * h);
            let expected = radius * angle.sin() / (1.0 - 1.0 / radius).sqrt();
            assert!((h / energy - expected).abs() / expected < 1e-5);
        }
    }
}
