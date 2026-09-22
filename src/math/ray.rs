//! Rayo: origen mas direccion.

use glam::Vec3;

/// Un rayo en espacio de mundo.
///
/// Direccion unitaria medida por la camara local. Photon la transforma
/// al marco de coordenadas de Schwarzschild antes de integrar.
#[derive(Clone, Copy, Debug)]
pub struct Ray {
    /// Punto de partida.
    pub origin: Vec3,
    /// Direccion de avance, normalizada.
    pub dir: Vec3,
}

impl Ray {
    /// Construye el rayo normalizando la direccion.
    pub fn new(origin: Vec3, dir: Vec3) -> Self {
        Self {
            origin,
            dir: dir.normalize_or_zero(),
        }
    }
}
