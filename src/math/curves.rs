//! Curvas de remapeo escalar.
//!
//! Son las funciones que en GLSL vienen de fabrica y en Rust no. Todas son
//! puras, baratas y se llaman millones de veces por frame, asi que estan
//! marcadas `#[inline]`.

/// Recorta a `[0, 1]`. El `clamp` de GLSL con 0 y 1.
#[inline]
pub fn saturate(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

/// Interpolacion suave de Hermite entre `edge0` y `edge1`.
///
/// Devuelve 0 por debajo de `edge0`, 1 por encima de `edge1`, y `3t^2 - 2t^3`
/// en el medio. Derivada nula en los extremos, por eso no se ve el corte.
#[inline]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = saturate((x - edge0) / (edge1 - edge0));
    t * t * (3.0 - 2.0 * t)
}

/// Lleva `x` del rango `[in_min, in_max]` al rango `[out_min, out_max]`,
/// linealmente y sin recortar.
#[inline]
pub fn remap(x: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    let t = (x - in_min) / (in_max - in_min);
    out_min + t * (out_max - out_min)
}
