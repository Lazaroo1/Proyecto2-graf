//! Utilidades matematicas del proyecto.
//!
//! Vectores y matrices propios, SDFs, ruido, curvas de remapeo y conversion
//! de temperatura a color. Todas las operaciones usan Rust estandar.

pub mod blackbody;
pub mod curves;
pub mod noise;
pub mod ray;
pub mod sdf;
mod vector;

pub use ray::Ray;
pub use vector::{Mat3, Vec2, Vec3};
