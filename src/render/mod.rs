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
use crate::scene::{
    disk::Medium, endurance, endurance::Ship, lighting::Lighting, skybox::Skybox, stars::SkyFrame,
};
use crate::version::Version;
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
    version: Version,
    /// El skybox se genera la primera vez que se usa: variante o Endurance.
    skybox: Option<Skybox>,
    /// Luz del entorno en la nave: se captura al entrar a la Endurance.
    lighting: Option<Lighting>,
    ship: Option<Ship>,
    ship_spin: f32,
    letterbox: bool,
    /// Contador de frames para el grano de pelicula.
    frame_index: u32,
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
            version: Version::Variante,
            skybox: None,
            lighting: None,
            ship: None,
            ship_spin: 0.0,
            letterbox: true,
            frame_index: 0,
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
        let endurance = self.version == Version::Endurance;
        let enhanced = self.version.enhanced();

        if enhanced {
            self.gas.update_preview(&self.noise, time);
            if self.skybox.is_none() {
                self.skybox = Some(Skybox::generate(&self.noise));
            }
        } else {
            self.gas.update(&self.noise, time);
        }
        if endurance {
            self.prepare_endurance(time);
        }
        let observer = camera.position().length();
        let render_height = self.frame.height();
        let scene = raymarch::SceneFrame {
            noise: &self.noise,
            gas: &self.gas,
            enhanced,
            sky: if enhanced {
                SkyFrame::galaxy(time, observer)
            } else {
                SkyFrame::new(time, observer)
            },
            skybox: if enhanced { self.skybox.as_ref() } else { None },
            sky_gain: if endurance {
                config::ENDURANCE_SKY_GAIN
            } else {
                config::VARIANT_SKY_GAIN
            },
            ship: if endurance { self.ship.as_ref() } else { None },
            medium: match self.version {
                Version::Original => Medium::ORIGINAL,
                Version::Variante => Medium::VARIANT,
                Version::Endurance => Medium::ENDURANCE,
            },
            pixel_angle: 2.0 * (camera.fov_y * 0.5).tan() / render_height as f32,
            letterbox: endurance && self.letterbox,
            tag: [
                self.version.index(),
                if endurance { self.ship_spin.to_bits() } else { 0 },
            ],
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
        let exposure = if endurance {
            config::ENDURANCE_EXPOSURE
        } else {
            config::EXPOSURE
        };
        self.bloom
            .process(&self.frame, config::BLOOM_THRESHOLD / exposure);
        let bloom_intensity = match self.version {
            Version::Original => config::BLOOM_INTENSITY,
            Version::Variante => config::PREVIEW_BLOOM_INTENSITY,
            Version::Endurance => config::ENDURANCE_BLOOM_INTENSITY,
        };
        self.bloom.composite(&mut self.frame, bloom_intensity);

        if endurance {
            self.bloom.composite_glare(&mut self.frame, config::GLARE_INTENSITY);
            self.bloom.streaks(config::STREAK_THRESHOLD / exposure);
            self.bloom
                .composite_streaks(&mut self.frame, config::STREAK_INTENSITY);
            self.frame_index = self.frame_index.wrapping_add(1);
            tonemap::apply_film(
                &mut self.frame,
                exposure,
                config::GAMMA,
                self.letterbox,
                self.frame_index,
            );
        } else {
            tonemap::apply(&mut self.frame, exposure, config::GAMMA);
        }

        self.frame
            .present_argb(self.window_width, self.window_height)
    }

    pub fn set_version(&mut self, version: Version) {
        self.version = version;
    }

    /// Angulo del anillo de la nave. Cambiarlo invalida el cache de rayos.
    pub fn set_ship_spin(&mut self, spin: f32) {
        if spin != self.ship_spin {
            self.ship_spin = spin;
            self.ship = None;
        }
    }

    pub fn toggle_letterbox(&mut self) {
        self.letterbox = !self.letterbox;
    }

    /// La luz del entorno y la nave se construyen solo si se usan. Capturar
    /// la luz traza unas dos mil geodesicas desde la nave, una sola vez: no
    /// depende del giro del anillo.
    fn prepare_endurance(&mut self, time: f32) {
        if self.lighting.is_none() {
            let capture = raymarch::SceneFrame {
                noise: &self.noise,
                gas: &self.gas,
                enhanced: true,
                sky: SkyFrame::new(time, endurance::ship_center().length()),
                skybox: None,
                sky_gain: 0.0,
                ship: None,
                medium: Medium::ENDURANCE,
                pixel_angle: 0.0,
                letterbox: false,
                tag: [0, 0],
            };
            let samples = raymarch::capture_environment(
                &capture,
                endurance::ship_center(),
                config::SHIP_LIGHT_SAMPLES,
            );
            self.lighting = Some(Lighting::from_samples(&samples));
        }
        if self.ship.is_none() {
            if let Some(lighting) = &self.lighting {
                self.ship = Some(Ship::new(self.ship_spin, lighting.clone()));
            }
        }
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
