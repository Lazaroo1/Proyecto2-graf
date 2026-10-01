//! Luz del entorno de la nave: lo que veria un observador en su posicion.
//!
//! La radiancia se captura con geodesicas completas desde el centro de la
//! nave (ver `raymarch::capture_environment`), asi que incluye la lente: el
//! disco directamente debajo, su lado lejano curvado sobre la sombra y el
//! anillo de fotones. Con esas muestras se arman dos cosas:
//!
//! - **Luces clave**: las zonas mas brillantes del cielo de la nave, como
//!   luces direccionales con sombra suave. Dan el contraste y los brillos.
//! - **Armonicos esfericos de orden 2** con el resto: nueve coeficientes por
//!   canal que dan la irradiancia difusa para cualquier normal con una suma
//!   (Ramamoorthi y Hanrahan, 2001). Con la oclusion ambiental hacen la luz
//!   suave del disco que llena la nave desde abajo.

use crate::math::Vec3;
use std::f32::consts::PI;

/// Direcciones de Fibonacci sobre la esfera: cobertura casi uniforme.
pub fn fibonacci(index: usize, count: usize) -> Vec3 {
    let y = 1.0 - 2.0 * (index as f32 + 0.5) / count as f32;
    let radius = (1.0 - y * y).max(0.0).sqrt();
    let angle = index as f32 * 2.399_963_1;
    Vec3::new(radius * angle.cos(), y, radius * angle.sin())
}

/// Base real de armonicos esfericos hasta l = 2.
fn basis(d: Vec3) -> [f32; 9] {
    [
        0.282_095,
        0.488_603 * d.y,
        0.488_603 * d.z,
        0.488_603 * d.x,
        1.092_548 * d.x * d.y,
        1.092_548 * d.y * d.z,
        0.315_392 * (3.0 * d.z * d.z - 1.0),
        1.092_548 * d.x * d.z,
        0.546_274 * (d.x * d.x - d.y * d.y),
    ]
}

/// Convolucion con el coseno por banda: pi, 2pi/3 y pi/4.
const BAND_WEIGHT: [f32; 9] = [
    PI,
    2.0 * PI / 3.0,
    2.0 * PI / 3.0,
    2.0 * PI / 3.0,
    PI / 4.0,
    PI / 4.0,
    PI / 4.0,
    PI / 4.0,
    PI / 4.0,
];

/// Zonas en que se agrupan las muestras para elegir las luces clave.
const BUCKETS: usize = 48;
const KEY_LIGHTS: usize = 3;

#[derive(Clone, Copy, Debug)]
pub struct KeyLight {
    /// Unitario, de la nave hacia la luz.
    pub direction: Vec3,
    /// Irradiancia sobre una superficie perpendicular: radiancia por angulo solido.
    pub irradiance: Vec3,
    /// Factor de penumbra de la sombra suave: mas alto es una luz mas chica.
    pub softness: f32,
    /// Radio angular aproximado, para ensanchar los brillos de superficies lisas.
    pub angular_radius: f32,
}

#[derive(Clone, Debug)]
pub struct Lighting {
    /// Entorno sin las luces clave, para el difuso.
    residual: [Vec3; 9],
    /// Entorno completo, para reflejos borrosos de superficies rugosas.
    full: [Vec3; 9],
    keys: Vec<KeyLight>,
}

impl Lighting {
    /// `radiance[i]` llega desde `fibonacci(i, radiance.len())`.
    pub fn from_samples(radiance: &[Vec3]) -> Self {
        let count = radiance.len().max(1);
        let solid_angle = 4.0 * PI / count as f32;
        let centers: Vec<Vec3> = (0..BUCKETS).map(|i| fibonacci(i, BUCKETS)).collect();
        let bucket_of = |direction: Vec3| {
            (0..BUCKETS)
                .max_by(|&a, &b| direction.dot(centers[a]).total_cmp(&direction.dot(centers[b])))
                .unwrap_or(0)
        };
        let mut energy = [Vec3::ZERO; BUCKETS];
        let mut weighted = [Vec3::ZERO; BUCKETS];
        let mut weight = [0.0_f32; BUCKETS];
        let mut assigned = Vec::with_capacity(count);
        for (i, &value) in radiance.iter().enumerate() {
            let direction = fibonacci(i, count);
            let b = bucket_of(direction);
            let w = value.luminance() * solid_angle;
            energy[b] += value * solid_angle;
            weighted[b] += direction * w;
            weight[b] += w;
            assigned.push(b);
        }

        // Las zonas mas brillantes, si aportan una fraccion apreciable.
        let total: f32 = weight.iter().sum();
        let mut order: Vec<usize> = (0..BUCKETS).collect();
        order.sort_by(|&a, &b| weight[b].total_cmp(&weight[a]));
        let chosen: Vec<usize> = order
            .into_iter()
            .take(KEY_LIGHTS)
            .filter(|&b| weight[b] > 0.04 * total && weight[b] > 0.0)
            .collect();
        let keys = chosen
            .iter()
            .map(|&b| {
                // La concentracion de las direcciones dentro de la zona da su
                // tamano angular: largo medio del vector resultante.
                let mean = weighted[b] / weight[b];
                let concentration = mean.length().clamp(0.0, 1.0);
                let angular_radius = concentration.acos().max(0.02);
                KeyLight {
                    direction: mean.normalize_or_zero(),
                    irradiance: energy[b],
                    softness: (1.0 / angular_radius.tan()).clamp(2.0, 24.0),
                    angular_radius,
                }
            })
            .collect();

        let mut residual = [Vec3::ZERO; 9];
        let mut full = [Vec3::ZERO; 9];
        for (i, &value) in radiance.iter().enumerate() {
            let y = basis(fibonacci(i, count));
            let in_key = chosen.contains(&assigned[i]);
            for k in 0..9 {
                let projected = value * (y[k] * solid_angle);
                full[k] += projected;
                if !in_key {
                    residual[k] += projected;
                }
            }
        }
        Self { residual, full, keys }
    }

    /// Irradiancia difusa del entorno sin las luces clave, para la normal `n`.
    pub fn irradiance(&self, n: Vec3) -> Vec3 {
        evaluate(&self.residual, n)
    }

    /// Radiancia promedio de un lobulo coseno alrededor de `direction`: el
    /// reflejo borroso de una superficie rugosa.
    pub fn glossy(&self, direction: Vec3) -> Vec3 {
        evaluate(&self.full, direction) * (1.0 / PI)
    }

    pub fn keys(&self) -> &[KeyLight] {
        &self.keys
    }
}

fn evaluate(coefficients: &[Vec3; 9], n: Vec3) -> Vec3 {
    let y = basis(n);
    let mut sum = Vec3::ZERO;
    for k in 0..9 {
        sum += coefficients[k] * (BAND_WEIGHT[k] * y[k]);
    }
    sum.max(Vec3::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_sky_gives_pi_times_radiance_from_any_side() {
        let samples = vec![Vec3::splat(0.5); 4096];
        let lighting = Lighting::from_samples(&samples);
        for n in [Vec3::X, Vec3::Y, -Vec3::Z, Vec3::new(0.3, -0.8, 0.52).normalize()] {
            let e = lighting.glossy(n) * PI;
            assert!((e.x - 0.5 * PI).abs() < 0.02, "{e:?}");
        }
    }

    #[test]
    fn a_glowing_floor_lights_from_below_and_becomes_a_key_light() {
        // Disco luminoso debajo, cielo negro arriba: la cara de abajo recibe
        // casi pi*L y la de arriba casi nada.
        let count = 4096;
        let samples: Vec<Vec3> = (0..count)
            .map(|i| if fibonacci(i, count).y < 0.0 { Vec3::ONE } else { Vec3::ZERO })
            .collect();
        let lighting = Lighting::from_samples(&samples);
        let down = lighting.glossy(-Vec3::Y) * PI;
        let up = lighting.glossy(Vec3::Y) * PI;
        assert!((down.x - PI).abs() < 0.35 && up.x < 0.35, "{down:?} {up:?}");
        assert!(!lighting.keys().is_empty());
        assert!(lighting.keys().iter().all(|key| key.direction.y < 0.0));
        // Las luces clave salen del difuso: no se cuentan dos veces.
        assert!(lighting.irradiance(-Vec3::Y).x < down.x);
    }
}
