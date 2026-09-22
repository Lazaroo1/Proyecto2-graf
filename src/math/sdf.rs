//! Funciones de distancia con signo.
//!
//! Convencion: negativo adentro, positivo afuera, cero en la superficie.
//!
//! Queda una sola. La version anterior tenia ademas un toro, que modelaba el aro
//! de neblina alrededor del horizonte; ese aro desaparecio al integrar bien las
//! geodesicas, porque el halo que intentaba imitar resulto ser el anillo de
//! fotones y ese sale solo.

use glam::Vec3;

/// Distancia con signo a una esfera centrada en el origen.
///
/// El horizonte de eventos de Schwarzschild es exactamente eso: una esfera de
/// radio `rs` en estas coordenadas.
#[inline]
pub fn sd_sphere(p: Vec3, radius: f32) -> f32 {
    p.length() - radius
}
