//! Verificacion de la fisica contra resultados analiticos.
//!
//! Un integrador de geodesicas puede estar completamente roto y producir igual
//! una imagen que parece un agujero negro. La unica forma de saber si esta bien
//! es contrastarlo con numeros que la relatividad general predice de forma
//! cerrada, y que no dependen de ninguna eleccion del codigo.
//!
//! Se corre con `cargo run --release -- --verify`. Usa exactamente el mismo
//! `Photon` que el render: si estas pruebas pasan, la imagen esta integrada con
//! esa misma fisica.
//!
//! Las cuatro pruebas son independientes entre si y fallan de formas distintas,
//! que es lo que las hace utiles:
//!
//! - El **parametro de impacto critico** verifica la escala global de la
//!   curvatura. Si el coeficiente `3/2` de la aceleracion estuviera mal, este
//!   numero se corre y la sombra sale del tamano equivocado.
//! - Las **cantidades conservadas** verifican el integrador. Si `h^2` o `E`
//!   derivan, el paso es demasiado largo o el Runge-Kutta esta mal armado.
//! - La **esfera de fotones** verifica el equilibrio entre curvatura e inercia en
//!   el unico radio donde ese equilibrio es exacto.
//! - La **deflexion de campo debil** verifica el limite clasico: lejos del
//!   agujero tiene que reproducir el `4GM/(c^2 b)` de Einstein de 1915, que es la
//!   prediccion que se midio en el eclipse de 1919.

use glam::Vec3;

use crate::config;
use crate::scene::blackhole::Photon;
use crate::scene::relativity;

/// Pasos maximos de las pruebas. Mucho mas generoso que el del render, porque
/// aca se integra desde muy lejos y la precision importa mas que la velocidad.
const VERIFY_MAX_STEPS: usize = 20_000;

/// Corre todas las pruebas e informa el resultado.
pub fn run() -> bool {
    println!("Verificacion de la fisica (rs = 1, M = 0.5)\n");

    let mut all_passed = true;
    all_passed &= critical_impact_parameter();
    all_passed &= conserved_quantities();
    all_passed &= photon_sphere();
    all_passed &= weak_field_deflection();

    println!();
    if all_passed {
        println!("Todas las pruebas pasaron.");
    } else {
        println!("Hay pruebas que fallaron.");
    }
    all_passed
}

/// Imprime una linea de resultado y devuelve si paso.
fn report(name: &str, measured: f32, expected: f32, tolerance: f32, unit: &str) -> bool {
    let error = (measured - expected).abs() / expected.abs().max(1.0e-12);
    let passed = error <= tolerance;
    println!(
        "  [{}] {name}\n        medido {measured:.6} {unit}   esperado {expected:.6} {unit}   error {:.3}%",
        if passed { "ok" } else { "FALLA" },
        error * 100.0
    );
    passed
}

/// Integra un foton desde `origin` con direccion `direction` hasta que escapa
/// mas alla de `escape_radius` o lo captura el agujero.
///
/// El radio de escape es un parametro y no el del render a proposito. El render
/// corta en 45 rs porque a esa distancia la direccion ya no cambia lo suficiente
/// como para que se note en un pixel, pero medir una deflexion asintotica con ese
/// corte truncaria el resultado: falta la parte que se acumula de ahi al infinito.
fn integrate(origin: Vec3, direction: Vec3, escape_radius: f32) -> Option<Vec3> {
    let mut photon = Photon::from_camera(origin, direction);
    for _ in 0..VERIFY_MAX_STEPS {
        if photon.captured() {
            return None;
        }
        if photon.pos.length() > escape_radius && photon.vel.dot(photon.pos) > 0.0 {
            return Some(photon.direction());
        }
        let step = photon.step_length();
        photon.advance(step);
    }
    // Se quedo sin pasos sin resolverse: se trata como capturado, que es lo que
    // pasa con las orbitas que espiralean cerca de la esfera de fotones.
    None
}

/// Lanza un foton paralelo al eje Z con parametro de impacto `b`.
///
/// Lejos del agujero la geodesica es casi recta, asi que `|r x v|` coincide con
/// la distancia de esa recta al centro: el parametro de impacto.
fn shoot(impact_parameter: f32, start_radius: f32, escape_radius: f32) -> Option<Vec3> {
    integrate(
        Vec3::new(impact_parameter, 0.0, -start_radius),
        Vec3::new(0.0, 0.0, 1.0),
        escape_radius,
    )
}

/// Parametro de impacto critico: `b_c = 3 sqrt(3) M`.
///
/// Por debajo de `b_c` el foton cae; por encima, escapa. Ese valor es
/// exactamente el radio angular de la sombra que ve un observador lejano, y es la
/// razon por la que la sombra de un agujero negro se ve 2.6 veces mas grande que
/// su horizonte. Se busca por biseccion.
fn critical_impact_parameter() -> bool {
    let start_radius = 300.0;
    let (mut low, mut high) = (1.0_f32, 6.0_f32);

    for _ in 0..40 {
        let mid = 0.5 * (low + high);
        if shoot(mid, start_radius, config::ESCAPE_RADIUS).is_some() {
            high = mid; // escapo: el critico esta por debajo
        } else {
            low = mid; // cayo: el critico esta por encima
        }
    }

    // La posicion transversal a radio finito no es el parametro de impacto:
    // comparar la constante L/E, incluyendo el marco local de la camara.
    let photon = Photon::from_camera(Vec3::new(0.5 * (low + high), 0.0, -start_radius), Vec3::Z);
    let h_sq = relativity::angular_momentum(photon.pos, photon.vel).length_squared();
    let impact = h_sq.sqrt() / relativity::photon_energy(photon.pos, photon.vel, h_sq);
    report(
        "Parametro de impacto critico  b_c = 3*sqrt(3)*M",
        impact,
        config::SHADOW_RADIUS,
        0.01,
        "rs",
    )
}

/// Deriva de las cantidades conservadas a lo largo de una geodesica.
///
/// `h^2 = |r x v|^2` y `E^2 = |v|^2 - rs h^2 / r^3` son constantes exactas del
/// movimiento. Lo que se mide aca no es fisica sino calidad numerica: cuanto se
/// aparta el integrador de conservarlas.
///
/// Se elige un rayo que pasa rozando la esfera de fotones, que es el caso mas
/// exigente: es donde la aceleracion es mayor y donde el rayo da mas vueltas.
fn conserved_quantities() -> bool {
    let origin = Vec3::new(2.7, 0.0, -300.0);
    let mut photon = Photon::from_camera(origin, Vec3::new(0.0, 0.0, 1.0));

    let reference_h = relativity::angular_momentum(photon.pos, photon.vel).length_squared();
    let reference_e = relativity::photon_energy(photon.pos, photon.vel, reference_h);

    let mut worst_h: f32 = 0.0;
    let mut worst_e: f32 = 0.0;

    for _ in 0..VERIFY_MAX_STEPS {
        if photon.captured() || photon.escaped() {
            break;
        }
        let step = photon.step_length();
        photon.advance(step);

        let h = relativity::angular_momentum(photon.pos, photon.vel).length_squared();
        let e = relativity::photon_energy(photon.pos, photon.vel, reference_h);
        worst_h = worst_h.max((h - reference_h).abs() / reference_h);
        worst_e = worst_e.max((e - reference_e).abs() / reference_e);
    }

    println!(
        "  [{}] Conservacion a lo largo de la geodesica\n        deriva maxima de h^2 {:.3e}   de E {:.3e}   (tolerancia 1e-3)",
        if worst_h < 1.0e-3 && worst_e < 1.0e-3 {
            "ok"
        } else {
            "FALLA"
        },
        worst_h,
        worst_e
    );
    worst_h < 1.0e-3 && worst_e < 1.0e-3
}

/// Orbita circular de luz en `r = 3M`.
///
/// Un foton lanzado tangencialmente exactamente ahi deberia quedarse ahi para
/// siempre: es el unico radio donde la curvatura equilibra a la inercia para algo
/// que se mueve a `c`. La orbita es inestable, asi que cualquier error de
/// integracion se amplifica exponencialmente. Es la prueba mas sensible de las
/// cuatro.
fn photon_sphere() -> bool {
    let radius = config::PHOTON_SPHERE_RADIUS;
    let mut photon = Photon::from_camera(Vec3::new(radius, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0));

    // Una vuelta completa. Con `|v| = 1` la longitud de arco es `2 pi r`.
    let target_arc = std::f32::consts::TAU * radius;
    let mut travelled = 0.0;
    let mut worst_radius = radius;

    while travelled < target_arc {
        let step = photon.step_length().min(0.01);
        photon.advance(step);
        travelled += step;

        let r = photon.pos.length();
        if (r - radius).abs() > (worst_radius - radius).abs() {
            worst_radius = r;
        }
        if photon.captured() {
            break;
        }
    }

    report(
        "Esfera de fotones: radio tras una vuelta completa",
        worst_radius,
        radius,
        0.02,
        "rs",
    )
}

/// Deflexion de la luz, con el primer termino post-Newtoniano.
///
/// El resultado famoso de Einstein, `alpha = 4GM/(c^2 b)`, es solo el termino
/// dominante de una serie:
///
/// ```text
///   alpha = 4M/b + (15 pi/4) (M/b)^2 + (128/3) (M/b)^3 + ...
/// ```
///
/// Ese primer termino es el doble de lo que da la gravedad newtoniana tratando a
/// la luz como una particula, y es la prediccion que se confirmo en el eclipse de
/// 1919. Pero solo es exacto en el limite `b >> M`: a 25 radios de Schwarzschild
/// el segundo termino ya vale casi un 6%, asi que compararse contra el primero
/// solo daria un fallo espurio. Se compara contra los dos.
///
/// Queda un error residual de fraccion de porciento por los radios finitos: el
/// rayo arranca a distancia finita y se mide a distancia finita, y en cada
/// extremo se pierde aproximadamente `2M/r` de deflexion.
fn weak_field_deflection() -> bool {
    let impact_parameter = 25.0;
    let start_radius = 4000.0;
    let escape_radius = 2500.0;

    let incoming = Vec3::new(0.0, 0.0, 1.0);
    let outgoing = match shoot(impact_parameter, start_radius, escape_radius) {
        Some(direction) => direction,
        None => {
            println!("  [FALLA] Deflexion de la luz: el foton no escapo");
            return false;
        }
    };

    let measured = incoming.dot(outgoing).clamp(-1.0, 1.0).acos();

    let ratio = config::BLACK_HOLE_MASS / impact_parameter;
    let first_order = 4.0 * ratio;
    let second_order = 15.0 * std::f32::consts::PI / 4.0 * ratio * ratio;
    let expected = first_order + second_order;

    let passed = report(
        "Deflexion de la luz  alpha = 4M/b + (15pi/4)(M/b)^2",
        measured,
        expected,
        0.02,
        "rad",
    );
    println!("        desglose: primer orden {first_order:.6}   segundo orden {second_order:.6}");
    passed
}

#[cfg(test)]
mod tests {
    #[test]
    fn schwarzschild_analytic_regressions() {
        assert!(super::run());
    }
}
