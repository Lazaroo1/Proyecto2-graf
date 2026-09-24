//! Pipeline de render.
//!
//! `Renderer` es dueno de todos los buffers y los reutiliza entre frames.
//! El orden de las etapas es:
//!
//! ```text
//!   raymarch   -> frame HDR crudo            (equivale al Buffer A del shader)
//!   accumulate -> mezcla con el historico    (la realimentacion del Buffer A)
//!   guardar historico                        <- antes del bloom, a proposito
//!   bloom      -> piramide + blur separable  (Buffers B, C, D)
//!   tonemap    -> exposicion, curva, gamma   (pasada Image)
//!   present_argb    -> escalar a ventana y empaquetar
//! ```
//!
//! El historico se guarda antes del bloom y no despues. Si se guardara despues,
//! el halo del frame anterior entraria como color base del siguiente, el bloom
//! se alimentaria de si mismo y la imagen se iria a blanco en pocos segundos.

pub mod accumulate;
pub mod bloom;
pub mod framebuffer;
pub mod raymarch;
pub mod tonemap;

use crate::camera::OrbitCamera;
use crate::config;
use crate::math::noise::NoiseTable;
use bloom::BloomChain;
use framebuffer::HdrBuffer;

/// Estado persistente del render.
pub struct Renderer {
    /// Tamano de la ventana. El frame final siempre sale con estas dimensiones.
    window_width: usize,
    window_height: usize,
    /// Escala de resolucion interna vigente.
    render_scale: f32,
    /// Frame en construccion, a resolucion de render.
    frame: HdrBuffer,
    /// Frame acumulado del paso anterior, antes del bloom y del tonemap.
    history: HdrBuffer,
    /// Piramide de bloom.
    bloom: BloomChain,
    /// Tabla de ruido, compartida por todos los hilos del raymarch.
    noise: NoiseTable,
    gas: crate::scene::disk::GasTexture,
    rays: raymarch::RayCache,
    previous_time: Option<f32>,
    enhanced: bool,
}

impl Renderer {
    /// Reserva todos los buffers y precomputa la tabla de ruido.
    ///
    /// Es la unica parte cara del arranque y corre una sola vez.
    pub fn new(window_width: usize, window_height: usize) -> Self {
        let scale = config::RENDER_SCALE_INITIAL;
        let (rw, rh) = framebuffer::scaled_dimensions(window_width, window_height, scale);

        Self {
            window_width,
            window_height,
            render_scale: scale,
            frame: HdrBuffer::new(rw, rh),
            history: HdrBuffer::new(rw, rh),
            bloom: BloomChain::new(window_width, window_height),
            noise: NoiseTable::new(config::NOISE_SEED),
            gas: crate::scene::disk::GasTexture::new(),
            rays: raymarch::RayCache::default(),
            previous_time: None,
            enhanced: true,
        }
    }

    /// Renderiza un frame y devuelve el buffer listo para `update_with_buffer`.
    ///
    /// `camera_moved` viene de `InputState` y controla dos cosas a la vez: la
    /// resolucion interna (baja mientras el usuario arrastra, para que la
    /// interaccion se sienta fluida) y el peso del historico en la acumulacion.
    pub fn render(&mut self, camera: &OrbitCamera, time: f32, camera_moved: bool) -> &[u32] {
        let target_scale = if camera_moved {
            config::RENDER_SCALE_DRAG
        } else {
            config::RENDER_SCALE_IDLE
        };
        let history_reset = self.set_render_scale(target_scale);

        if self.enhanced {
            self.gas.update_preview(&self.noise, time);
        } else {
            self.gas.update(&self.noise, time);
        }
        let scene = raymarch::SceneFrame {
            noise: &self.noise,
            gas: &self.gas,
            enhanced: self.enhanced,
            sky: crate::scene::stars::SkyFrame::new(time, camera.position().length()),
        };
        let view_changed = self
            .rays
            .render(&mut self.frame, camera, &scene, camera_moved);

        // Tras un cambio de resolucion el historico quedo en negro: mezclarlo
        // oscureceria el frame entero. Este frame sale crudo y el siguiente ya
        // tiene historico valido.
        if !history_reset && !view_changed && !camera_moved {
            if let Some(previous) = self.previous_time {
                accumulate::blend(
                    &mut self.frame,
                    &self.history,
                    accumulate::blend_factor(time - previous),
                );
            }
        }
        self.previous_time = Some(time);
        self.history.copy_from(&self.frame);

        // El umbral del bloom esta en fraccion del blanco de pantalla, asi que
        // hay que llevarlo al espacio del buffer dividiendo por la exposicion.
        self.bloom
            .process(&self.frame, config::BLOOM_THRESHOLD / config::EXPOSURE);
        self.bloom
            .composite(&mut self.frame, config::BLOOM_INTENSITY);

        tonemap::apply(&mut self.frame, config::EXPOSURE, config::GAMMA);

        self.frame
            .present_argb(self.window_width, self.window_height)
    }

    pub fn set_enhanced(&mut self, enhanced: bool) {
        self.enhanced = enhanced;
    }

    pub fn enhanced(&self) -> bool {
        self.enhanced
    }

    /// Cambia la resolucion interna, reasignando los buffers que dependen de ella.
    ///
    /// Devuelve `true` si hubo cambio, o sea, si el historico de acumulacion
    /// quedo invalidado.
    ///
    /// Salir temprano cuando la escala no cambio no es una microoptimizacion: si
    /// se reasignara todos los frames, el historico se perderia siempre y la
    /// acumulacion temporal nunca acumularia nada.
    fn set_render_scale(&mut self, scale: f32) -> bool {
        if (scale - self.render_scale).abs() < f32::EPSILON {
            return false;
        }
        self.render_scale = scale;

        let (rw, rh) = framebuffer::scaled_dimensions(self.window_width, self.window_height, scale);
        self.frame.resize(rw, rh);
        self.history.resize(rw, rh);
        // El radio del bloom permanece en pixeles de ventana al cambiar escala.
        true
    }
}
