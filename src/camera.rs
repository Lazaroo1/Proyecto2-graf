//! Camara orbital y generacion de rayos primarios.
//!
//! La camara siempre mira a su objetivo: el centro del agujero negro, o la
//! nave en la version Endurance. Su estado es esferico
//! (yaw, pitch, distancia) en vez de cartesiano, porque el control natural del
//! mouse es angular: arrastrar mueve angulos, el scroll mueve el radio.

use crate::math::{Vec2, Vec3};

use crate::config;
use crate::math::Ray;
use crate::scene::endurance;
use crate::version::Version;

/// Base ortonormal de la camara, en espacio de mundo.
///
/// Los tres vectores son unitarios y mutuamente perpendiculares, asi que la
/// matriz que forman es una rotacion pura: transformar un vector de espacio de
/// camara a mundo es una combinacion lineal, sin inversas ni divisiones.
#[derive(Clone, Copy, Debug)]
pub struct CameraBasis {
    /// Eje X de camara: hacia la derecha de la imagen.
    pub right: Vec3,
    /// Eje Y de camara: hacia arriba de la imagen.
    pub up: Vec3,
    /// Eje Z de camara: hacia donde mira.
    pub forward: Vec3,
}

/// Proyeccion preparada una vez por vista, sin trigonometria por pixel.
pub struct CameraProjection {
    origin: Vec3,
    basis: CameraBasis,
    width: f32,
    height: f32,
    half_width: f32,
    half_height: f32,
}

impl CameraProjection {
    pub fn ray_for_pixel(&self, x: usize, y: usize, jitter: Vec2) -> Ray {
        let u = (x as f32 + 0.5 + jitter.x) / self.width * 2.0 - 1.0;
        let v = 1.0 - (y as f32 + 0.5 + jitter.y) / self.height * 2.0;
        Ray::new(
            self.origin,
            self.basis.forward
                + self.basis.right * (u * self.half_width)
                + self.basis.up * (v * self.half_height),
        )
    }
}

/// Camara que orbita alrededor de un punto fijo.
pub struct OrbitCamera {
    /// Centro de la orbita: la singularidad, o la nave en la Endurance.
    pub target: Vec3,
    /// Angulo horizontal, en radianes.
    pub yaw: f32,
    /// Angulo vertical, en radianes. Acotado a +/- `CAMERA_PITCH_LIMIT`.
    pub pitch: f32,
    /// Radio de la orbita.
    pub distance: f32,
    /// Campo de vision vertical, en radianes.
    pub fov_y: f32,
    /// Giro de la camara sobre su propio eje, en radianes.
    pub roll: f32,
    /// Rango del zoom. Alrededor del agujero el minimo evita el horizonte;
    /// alrededor de la nave, atravesar su casco o acercarse demasiado al agujero.
    pub min_distance: f32,
    pub max_distance: f32,
    /// Fraccion de distancia por click de rueda, como exponente.
    pub zoom_sensitivity: f32,
    /// Altura minima de la camara en mundo. Junto a la nave evita meterse
    /// en el gas opaco del disco.
    pub floor: f32,
    /// Radio alrededor del agujero que la camara no cruza; 0 lo desactiva.
    pub keep_out: f32,
}

impl OrbitCamera {
    /// Camara en la pose inicial que define `config`.
    pub fn new() -> Self {
        Self {
            target: config::CAMERA_TARGET,
            yaw: config::CAMERA_YAW,
            pitch: config::CAMERA_PITCH,
            distance: config::CAMERA_DISTANCE,
            fov_y: config::FOV_DEGREES.to_radians(),
            roll: config::CAMERA_ROLL,
            min_distance: config::CAMERA_MIN_DISTANCE,
            max_distance: config::CAMERA_MAX_DISTANCE,
            zoom_sensitivity: config::ZOOM_SENSITIVITY,
            floor: f32::NEG_INFINITY,
            keep_out: 0.0,
        }
    }

    /// Pose de arranque de cada version.
    pub fn home(version: Version) -> Self {
        match version {
            Version::Endurance => {
                let mut camera = Self {
                    target: endurance::ship_center(),
                    yaw: behind_ship() + config::ENDURANCE_CAMERA_YAW,
                    pitch: config::ENDURANCE_CAMERA_PITCH,
                    distance: config::ENDURANCE_CAMERA_DISTANCE,
                    min_distance: config::ENDURANCE_CAMERA_MIN_DISTANCE,
                    max_distance: config::ENDURANCE_CAMERA_MAX_DISTANCE,
                    zoom_sensitivity: config::ENDURANCE_ZOOM_SENSITIVITY,
                    floor: config::ENDURANCE_CAMERA_FLOOR,
                    keep_out: config::ENDURANCE_CAMERA_KEEP_OUT,
                    ..Self::new()
                };
                camera.enforce_floor();
                camera
            }
            _ => Self::new(),
        }
    }

    /// Vistas de las teclas 1, 2 y 3.
    pub fn preset(version: Version, preset: u8) -> Self {
        let mut camera = Self::home(version);
        match (version, preset) {
            (Version::Endurance, 2) => {
                // Desde abajo y de costado: la nave iluminada por el mar de
                // nubes, recortada contra el arco de Gargantua y el cielo negro.
                camera.yaw = behind_ship() + 0.9;
                camera.pitch = -0.244;
                camera.distance = 0.17;
            }
            (Version::Endurance, 3) => {
                // Primer plano de la cupula desde abajo, con el arco del disco
                // y el anillo de fotones detras del nucleo.
                camera.yaw = behind_ship() + 0.35;
                camera.pitch = -0.21;
                camera.distance = 0.075;
            }
            (_, 2) => {
                camera.pitch = 30.0_f32.to_radians();
                camera.distance = 36.0;
            }
            (_, 3) => {
                camera.pitch = config::CAMERA_PITCH_LIMIT;
                camera.distance = 44.0;
            }
            _ => {}
        }
        camera.enforce_floor();
        camera
    }

    /// Posicion de la camara en mundo, de coordenadas esfericas a cartesianas.
    ///
    /// `pitch` es elevacion sobre el plano XZ, no angulo polar: por eso el
    /// `sin` va en Y y el `cos` escala el radio horizontal.
    pub fn position(&self) -> Vec3 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let direction = Vec3::new(cos_pitch * sin_yaw, sin_pitch, cos_pitch * cos_yaw);
        self.target + direction * self.reach(direction)
    }

    /// Distancia efectiva en la direccion de orbita: la pedida, salvo que la
    /// camara fuera a entrar a la esfera `keep_out` alrededor del agujero. Ahi
    /// se detiene en su borde, y al girar hacia otro lado recupera la distancia
    /// pedida.
    fn reach(&self, direction: Vec3) -> f32 {
        if self.keep_out <= 0.0 {
            return self.distance;
        }
        let half_b = self.target.dot(direction);
        let c = self.target.length_squared() - self.keep_out * self.keep_out;
        let discriminant = half_b * half_b - c;
        if c <= 0.0 || discriminant <= 0.0 {
            return self.distance;
        }
        let enter = -half_b - discriminant.sqrt();
        if enter > 0.0 {
            self.distance.min(enter)
        } else {
            self.distance
        }
    }

    /// Base ortonormal por Gram-Schmidt contra el up global.
    ///
    /// El tope de pitch de `config` garantiza que `forward` nunca sea paralelo a
    /// `Vec3::Y`, que es el caso donde este producto cruz degenera.
    pub fn basis(&self) -> CameraBasis {
        let forward = (self.target - self.position()).normalize_or_zero();
        let level_right = forward.cross(Vec3::Y).normalize_or_zero();
        let level_up = level_right.cross(forward);

        // Giro sobre el eje de vision. El disco queda en diagonal en vez de
        // horizontal, que es como estaba encuadrada la version original y lo que
        // deja ver a la vez el arco de arriba y el de abajo sin que ninguno se
        // vaya de cuadro. Como es una rotacion en el plano perpendicular a
        // `forward`, la base sigue siendo ortonormal.
        let (sin_roll, cos_roll) = self.roll.sin_cos();
        let right = level_right * cos_roll + level_up * sin_roll;
        let up = level_up * cos_roll - level_right * sin_roll;

        CameraBasis { right, up, forward }
    }

    /// Prepara una proyeccion local para toda la vista. El jitter subpixel
    /// se aplica despues, sin recalcular senos, cosenos o la base por rayo.
    pub fn projector(&self, width: usize, height: usize) -> CameraProjection {
        let aspect = width as f32 / height as f32;
        let half_height = (self.fov_y * 0.5).tan();
        CameraProjection {
            origin: self.position(),
            basis: self.basis(),
            width: width as f32,
            height: height as f32,
            half_width: half_height * aspect,
            half_height,
        }
    }

    /// Aplica un delta de orbita, en radianes, recortando el pitch.
    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch = (self.pitch + delta_pitch)
            .clamp(-config::CAMERA_PITCH_LIMIT, config::CAMERA_PITCH_LIMIT);
        self.enforce_floor();
    }

    /// Sube el pitch lo necesario para que la camara no quede bajo `floor`.
    fn enforce_floor(&mut self) {
        let lowest = (self.floor - self.target.y) / self.distance;
        if lowest > -1.0 {
            let pitch = lowest.min(1.0).asin();
            self.pitch = self.pitch.max(pitch).min(config::CAMERA_PITCH_LIMIT);
        }
    }

    /// Aplica zoom. `delta` positivo acerca.
    ///
    /// El factor es `exp(-delta * sensibilidad)` y no `1 - delta * sensibilidad`.
    /// La version lineal tiene dos problemas: no es simetrica, asi que acercar y
    /// alejar la misma cantidad no devuelve a la distancia original, y con un
    /// delta grande el factor cruza el cero y la distancia se va a valores sin
    /// sentido. La exponencial siempre es positiva, compone bien (dos pasos
    /// seguidos equivalen a uno del doble) y se siente igual de rapida de cerca
    /// que de lejos.
    pub fn zoom(&mut self, delta: f32) {
        let factor = (-delta * self.zoom_sensitivity).exp();
        self.distance = (self.distance * factor).clamp(self.min_distance, self.max_distance);
        self.enforce_floor();
    }
}

/// Yaw de la camara cuando queda detras de la nave, mirando hacia donde vuela.
fn behind_ship() -> f32 {
    let back = -endurance::ship_forward();
    back.x.atan2(back.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endurance_camera_stays_above_the_gas_and_zooms_into_the_ship() {
        let mut camera = OrbitCamera::home(Version::Endurance);
        camera.orbit(0.0, -1.4);
        assert!(camera.position().y >= config::ENDURANCE_CAMERA_FLOOR - 1e-5);
        camera.zoom(1000.0);
        assert_eq!(camera.distance, config::ENDURANCE_CAMERA_MIN_DISTANCE);
        assert!(camera.distance > 1.6 * config::SHIP_SCALE, "no atraviesa el casco");
        // La camara de arranque mira la nave por detras, hacia el agujero.
        let home = OrbitCamera::home(Version::Endurance);
        let look = (home.target - home.position()).normalize();
        assert!(look.dot(-home.target.normalize()) > 0.5);
    }

    #[test]
    fn zooming_far_out_never_crosses_into_the_black_hole() {
        let mut camera = OrbitCamera::home(Version::Endurance);
        camera.zoom(-1000.0);
        assert_eq!(camera.distance, config::ENDURANCE_CAMERA_MAX_DISTANCE);
        // Girar la orbita entera: en ninguna direccion la camara entra a la
        // esfera de exclusion, aunque la distancia pedida la atravesaria.
        let mut closest = f32::INFINITY;
        for _ in 0..720 {
            camera.orbit(0.0087, 0.0);
            closest = closest.min(camera.position().length());
        }
        assert!(closest >= config::ENDURANCE_CAMERA_KEEP_OUT - 1e-3, "{closest}");
        // Alrededor del agujero la camara no cambia: sin esfera de exclusion.
        let classic = OrbitCamera::new();
        assert_eq!(classic.keep_out, 0.0);
        assert!((classic.position().length() - classic.distance).abs() < 1e-4);
    }
}
