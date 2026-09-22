//! Unidades geometricas: rs = c = 1, M = 1/2.
//! Separar constantes de relatividad de parametros del gas y de la camara.
use glam::Vec3;

pub const WINDOW_WIDTH: usize = 960;
pub const WINDOW_HEIGHT: usize = 540;
pub const WINDOW_TITLE: &str = "Gargantua | Schwarzschild - CPU";
pub const TARGET_FPS: usize = 60;
pub const RENDER_SCALE_IDLE: f32 = 1.0;
pub const RENDER_SCALE_DRAG: f32 = 0.33;
pub const RENDER_SCALE_INITIAL: f32 = RENDER_SCALE_DRAG;
pub const MIN_RENDER_DIMENSION: usize = 16;
/// Muestras espaciales reutilizables; todas se sombrean al tiempo actual.
pub const SPATIAL_SAMPLES: usize = 2;

// Fisica de Schwarzschild, no ajustes esteticos.
pub const SCHWARZSCHILD_RADIUS: f32 = 1.0;
pub const BLACK_HOLE_MASS: f32 = 0.5;
pub const PHOTON_SPHERE_RADIUS: f32 = 1.5;
pub const SHADOW_RADIUS: f32 = 2.598_076;
pub const DISK_INNER_RADIUS: f32 = 3.0;

// Precision de la integracion (parametro afin).
pub const MAX_STEPS: usize = 512;
pub const STEP_ANGLE_TOLERANCE: f32 = 0.06;
pub const STEP_RADIUS_FRACTION: f32 = 0.10;
pub const MIN_STEP: f32 = 0.015;
pub const MAX_STEP: f32 = 5.0;
pub const ESCAPE_RADIUS: f32 = 60.0;
pub const MIN_TRANSMITTANCE: f32 = 0.004;

// Modelo de disco delgado termico (perfil newtoniano con borde ISCO).
pub const DISK_OUTER_RADIUS: f32 = 13.0;
pub const DISK_FADE_RADIUS: f32 = 7.5;
pub const DISK_INNER_FADE: f32 = 1.08;
/// Maximo de x^-3/4 (1-x^-1/2)^1/4. No es Novikov-Thorne completo.
pub const THIN_DISK_PROFILE_PEAK: f32 = 0.487_872;
pub const DISK_TEMPERATURE_PEAK: f32 = 7000.0;
pub const DISK_BRIGHTNESS: f32 = 1.0;

// Estructura procedural del plasma: no se resuelven ecuaciones MHD.
pub const DISK_NOISE_SCALE: f32 = 1.5;
pub const DISK_NOISE_RADIAL: f32 = 0.9;
pub const DISK_NOISE_ANGULAR: f32 = 5.0;
pub const DISK_TURBULENCE_OCTAVES: u32 = 5;
pub const DISK_TURBULENCE_AMOUNT: f32 = 0.96;
pub const DISK_TURBULENCE_CONTRAST: f32 = 8.0;
pub const DISK_FLOW_PERIOD: f32 = 5.0;
pub const DISK_FLOW_STRIDE: f32 = 17.3;
/// Unidades rs/c por segundo de animacion. No cambia la velocidad fisica.
pub const DISK_TIME_SCALE: f32 = 8.0;
/// Disco geometricamente delgado, con perfil gaussiano vertical.
pub const DISK_HEIGHT_RATIO: f32 = 0.004;
pub const DISK_OPTICAL_DEPTH: f32 = 8.0;
pub const DISK_ATMOSPHERE_HEIGHT_RATIO: f32 = 0.035;
pub const DISK_ATMOSPHERE_OPTICAL_DEPTH: f32 = 0.035;
pub const DISK_VOLUME_STEP: f32 = 0.22;
pub const GAS_TEXTURE_WIDTH: usize = 512;
pub const GAS_TEXTURE_HEIGHT: usize = 256;

// Fondo discreto, visible sin competir con el disco.
pub const STAR_GRID: f32 = 900.0;
pub const STAR_DENSITY: f32 = 0.010;
pub const STAR_SIZE: f32 = 0.13;
pub const STAR_BRIGHTNESS: f32 = 0.07;
pub const STAR_TEMPERATURE_MIN: f32 = 2800.0;
pub const STAR_TEMPERATURE_MAX: f32 = 12000.0;
pub const NEBULA_BRIGHTNESS: f32 = 0.00008;

// Observador estatico; elevacion pequena para ver los dos arcos de la lente.
pub const CAMERA_TARGET: Vec3 = Vec3::ZERO;
pub const FOV_DEGREES: f32 = 34.0;
pub const CAMERA_DISTANCE: f32 = 21.5;
pub const CAMERA_MIN_DISTANCE: f32 = 2.2;
pub const CAMERA_MAX_DISTANCE: f32 = 120.0;
pub const CAMERA_ROLL: f32 = 0.0;
pub const CAMERA_YAW: f32 = 0.6;
pub const CAMERA_PITCH: f32 = 0.035;
pub const CAMERA_PITCH_LIMIT: f32 = 1.52;
pub const ORBIT_SENSITIVITY: f32 = 0.006;
pub const ZOOM_SENSITIVITY: f32 = 0.12;
pub const KEY_ORBIT_SPEED: f32 = 1.1;
pub const KEY_ZOOM_SPEED: f32 = 6.0;
pub const MAX_FRAME_DELTA: f32 = 0.25;

// La historia dura milisegundos, no segundos de gas superpuesto.
pub const ACCUM_TIME_CONSTANT: f32 = 0.045;
pub const ACCUM_MAX_WEIGHT: f32 = 0.72;

// Respuesta optica de la camara; no interviene en las geodesicas.
pub const BLOOM_MIP_COUNT: usize = 6;
pub const BLOOM_BASE_SCALE: f32 = 0.5;
pub const BLOOM_BLUR_RADIUS: usize = 4;
pub const BLOOM_BLUR_SIGMA: f32 = 2.0;
pub const BLOOM_THRESHOLD: f32 = 0.6;
pub const BLOOM_MIP_FALLOFF: f32 = 0.72;
pub const BLOOM_INTENSITY: f32 = 0.38;
pub const HDR_CEILING: f32 = 150.0;
pub const EXPOSURE: f32 = 5.0;
pub const GAMMA: f32 = 2.2;
pub const NOISE_TABLE_SIZE: usize = 256;
pub const NOISE_SEED: u32 = 0x9E37_79B9;
