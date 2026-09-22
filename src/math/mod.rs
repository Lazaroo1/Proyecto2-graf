//! Utilidades matematicas del proyecto.
//!
//! Lo que glam ya da (vectores, matrices, lerp, longitudes) se usa directo. Aca
//! solo vive lo que glam no trae: SDFs, ruido, curvas de remapeo tipicas de
//! shader, y la conversion de temperatura a color.

pub mod blackbody;
pub mod curves;
pub mod noise;
pub mod ray;
pub mod sdf;

pub use ray::Ray;
