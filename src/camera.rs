//! Camara orbital y generacion de rayos primarios.
//!
//! La camara siempre mira al centro del agujero negro. Su estado es esferico
//! (yaw, pitch, distancia) en vez de cartesiano, porque el control natural del
//! mouse es angular: arrastrar mueve angulos, el scroll mueve el radio.

use glam::{Vec2, Vec3};

use crate::config;
use crate::math::Ray;

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
    /// Centro de la orbita. Aca es siempre la singularidad.
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
        }
    }

    /// Posicion de la camara en mundo, de coordenadas esfericas a cartesianas.
    ///
    /// `pitch` es elevacion sobre el plano XZ, no angulo polar: por eso el
    /// `sin` va en Y y el `cos` escala el radio horizontal.
    pub fn position(&self) -> Vec3 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let offset = Vec3::new(cos_pitch * sin_yaw, sin_pitch, cos_pitch * cos_yaw) * self.distance;
        self.target + offset
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
    }

    /// Aplica zoom. `delta` positivo acerca.
    ///
    /// El zoom es multiplicativo, no aditivo: asi se siente igual de rapido de
    /// cerca que de lejos.
    pub fn zoom(&mut self, delta: f32) {
        let factor = 1.0 - delta * config::ZOOM_SENSITIVITY;
        self.distance = (self.distance * factor)
            .clamp(config::CAMERA_MIN_DISTANCE, config::CAMERA_MAX_DISTANCE);
    }
}
