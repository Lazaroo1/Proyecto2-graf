//! Estado de mouse y teclado, y su mapeo a movimientos de camara.
//!
//! Este modulo no toca la camara directamente: acumula los deltas del frame y
//! los expone ya normalizados, sin importar de que dispositivo vinieron. El
//! mouse y el teclado terminan alimentando los mismos dos campos.
//!
//! La razon de no aplicarlos directo es que el flag `camera_moved` lo necesitan
//! tambien el renderer (para bajar la resolucion) y el acumulador temporal (para
//! bajar el peso del historico), no solo la camara.
//!
//! # Teclado y tiempo
//!
//! El mouse da desplazamientos: cuanto se movio desde el frame anterior. El
//! teclado da estados: la tecla esta apretada o no. Para que las dos cosas se
//! sientan igual, lo que aporta el teclado se multiplica por el tiempo del
//! frame. Sin eso la camara giraria al triple de velocidad a 45 FPS que a 15, y
//! como la resolucion interna baja justamente mientras uno mueve la camara, el
//! framerate cambia todo el tiempo.

use glam::Vec2;
use minifb::{Key, MouseButton, MouseMode, Window};

use crate::camera::OrbitCamera;
use crate::config;

/// Deltas de entrada acumulados en el frame actual.
pub struct InputState {
    /// Posicion del mouse en el frame anterior, para calcular el arrastre.
    /// `None` si todavia no hubo ningun frame con el mouse dentro de la ventana.
    last_mouse: Option<Vec2>,
    /// Si el boton izquierdo esta apretado en este frame.
    pub dragging: bool,
    /// Orbita pedida en este frame, en radianes: `x` es yaw, `y` es pitch.
    /// Suma lo que aportaron el arrastre del mouse y las teclas.
    pub orbit_delta: Vec2,
    /// Zoom pedido en este frame, en clicks de rueda equivalentes.
    /// Positivo acerca.
    pub zoom_delta: f32,
    /// Si el usuario pidio resetear la camara a la pose inicial.
    pub reset_requested: bool,
    pub preset: Option<u8>,
}

impl InputState {
    /// Estado vacio, sin historial de mouse.
    pub fn new() -> Self {
        Self {
            last_mouse: None,
            dragging: false,
            orbit_delta: Vec2::ZERO,
            zoom_delta: 0.0,
            reset_requested: false,
            preset: None,
        }
    }

    /// Lee la ventana y actualiza los deltas del frame.
    ///
    /// `dt` es la duracion del frame anterior, en segundos. Hay que llamar a
    /// esto exactamente una vez por frame, antes de `apply_to`.
    pub fn poll(&mut self, window: &Window, dt: f32) {
        self.orbit_delta = Vec2::ZERO;
        self.zoom_delta = 0.0;
        self.reset_requested = window.is_key_pressed(Key::R, minifb::KeyRepeat::No);
        self.preset = [Key::Key1, Key::Key2, Key::Key3]
            .iter()
            .position(|&key| window.is_key_pressed(key, minifb::KeyRepeat::No))
            .map(|i| i as u8 + 1);

        self.poll_mouse(window);
        self.poll_keyboard(window, dt);
    }

    /// Arrastre y rueda.
    fn poll_mouse(&mut self, window: &Window) {
        let was_dragging = self.dragging;
        self.dragging = window.get_mouse_down(MouseButton::Left);

        // `MouseMode::Pass` devuelve la posicion aunque el cursor se salga de la
        // ventana: si no, al arrastrar rapido el delta se corta en el borde.
        let current = window
            .get_mouse_pos(MouseMode::Pass)
            .map(|(x, y)| Vec2::new(x, y));

        if let (Some(prev), Some(now)) = (self.last_mouse, current) {
            // Solo cuenta el arrastre si el boton ya estaba apretado el frame
            // anterior. En el frame del click, `prev` es de antes de apretar y
            // el delta seria un salto espurio.
            if self.dragging && was_dragging {
                let drag = now - prev;
                // El yaw va invertido a proposito: arrastrar a la derecha tiene
                // que sentirse como empujar la escena a la derecha, no como
                // girar la camara a la derecha.
                self.orbit_delta += Vec2::new(-drag.x, drag.y) * config::ORBIT_SENSITIVITY;
            }
        }
        self.last_mouse = current;

        if let Some((_, vertical)) = window.get_scroll_wheel() {
            self.zoom_delta += vertical;
        }
    }

    /// WASD para orbitar, flechas verticales para acercar y alejar.
    ///
    /// Todo va escalado por `dt`, asi que la velocidad de giro no depende del
    /// framerate.
    fn poll_keyboard(&mut self, window: &Window, dt: f32) {
        let mut orbit = Vec2::ZERO;
        if window.is_key_down(Key::A) {
            orbit.x += 1.0;
        }
        if window.is_key_down(Key::D) {
            orbit.x -= 1.0;
        }
        if window.is_key_down(Key::W) {
            orbit.y += 1.0;
        }
        if window.is_key_down(Key::S) {
            orbit.y -= 1.0;
        }
        self.orbit_delta += orbit * config::KEY_ORBIT_SPEED * dt;

        let mut zoom = 0.0;
        if window.is_key_down(Key::Up) {
            zoom += 1.0;
        }
        if window.is_key_down(Key::Down) {
            zoom -= 1.0;
        }
        self.zoom_delta += zoom * config::KEY_ZOOM_SPEED * dt;
    }

    /// Si este frame movio la camara de alguna forma.
    ///
    /// Es la senal que dispara la bajada de resolucion interna y la bajada del
    /// peso del historico en la acumulacion temporal. Que las teclas tambien la
    /// enciendan no es un detalle: mientras se mantiene una tecla apretada, el
    /// render baja a un tercio y el giro se ve fluido.
    pub fn camera_moved(&self) -> bool {
        self.reset_requested
            || self.preset.is_some()
            || self.zoom_delta != 0.0
            || self.orbit_delta != Vec2::ZERO
    }

    /// Traslada los deltas del frame a la camara.
    pub fn apply_to(&self, camera: &mut OrbitCamera) {
        if let Some(preset) = self.preset {
            *camera = OrbitCamera::new();
            match preset {
                2 => {
                    camera.pitch = 30.0_f32.to_radians();
                    camera.distance = 36.0;
                }
                3 => {
                    camera.pitch = config::CAMERA_PITCH_LIMIT;
                    camera.distance = 44.0;
                }
                _ => {}
            }
            return;
        }
        if self.reset_requested {
            *camera = OrbitCamera::new();
            return;
        }
        if self.orbit_delta != Vec2::ZERO {
            camera.orbit(self.orbit_delta.x, self.orbit_delta.y);
        }
        if self.zoom_delta != 0.0 {
            camera.zoom(self.zoom_delta);
        }
    }
}
