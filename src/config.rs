//! Unidades geometricas: rs = c = 1, M = 1/2.
//! Separar constantes de relatividad de parametros del gas y de la camara.
use crate::math::Vec3;

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
/// Variante de color: un poco mas calida, con el mismo g relativista.
pub const PREVIEW_DISK_TEMPERATURE: f32 = 6500.0;
/// Atmosfera de la variante: mas gas visible por encima de la fotosfera.
pub const PREVIEW_ATMOSPHERE_HEIGHT_RATIO: f32 = 0.028;
pub const PREVIEW_ATMOSPHERE_OPTICAL_DEPTH: f32 = 0.10;
pub const DISK_BRIGHTNESS: f32 = 1.0;
// Envoltura exterior tenue. Su fuente gris aproxima dispersion de luz;
// no se presenta como emision termica de gas frio ni como GRMHD.
pub const OUTER_GAS_START: f32 = 6.0;
pub const OUTER_GAS_PEAK: f32 = 10.0;
pub const OUTER_GAS_FADE: f32 = 12.0;
pub const OUTER_GAS_END: f32 = 19.0;
pub const OUTER_GAS_HEIGHT_RATIO: f32 = 0.028;
pub const OUTER_GAS_OPTICAL_DEPTH: f32 = 0.30;
pub const OUTER_GAS_BRIGHTNESS: f32 = 0.045;
pub const OUTER_GAS_LIGHT_FALLOFF: f32 = 0.22;

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
/// Campo cilindrico 3D de la variante: azimut, radio y altura.
pub const PREVIEW_GAS_TEXTURE_WIDTH: usize = 384;
pub const PREVIEW_GAS_TEXTURE_HEIGHT: usize = 192;
pub const PREVIEW_GAS_TEXTURE_LAYERS: usize = 9;
pub const PREVIEW_GAS_TURBULENCE_OCTAVES: u32 = 4;
pub const PREVIEW_GAS_TURBULENCE_CONTRAST: f32 = 11.0;
pub const PREVIEW_GAS_NOISE_ANGULAR: f32 = 3.2;
pub const PREVIEW_GAS_NOISE_RADIAL: f32 = 1.3;
/// Cizalla del patron con la altura, en radianes por altura de escala.
pub const PREVIEW_GAS_VERTICAL_SHEAR: f32 = 0.18;

// Fondo discreto, visible sin competir con el disco.
pub const STAR_GRID: f32 = 900.0;
pub const STAR_DENSITY: f32 = 0.010;
pub const STAR_SIZE: f32 = 0.13;
pub const STAR_BRIGHTNESS: f32 = 0.07;
pub const STAR_TEMPERATURE_MIN: f32 = 2800.0;
pub const STAR_TEMPERATURE_MAX: f32 = 12000.0;
pub const NEBULA_BRIGHTNESS: f32 = 0.00008;
/// Campo nuevo: estrellas resueltas para hacer visible la lente.
pub const PREVIEW_STAR_GRID: f32 = 250.0;
pub const PREVIEW_STAR_DENSITY: f32 = 0.04;
pub const PREVIEW_STAR_SIZE: f32 = 0.13;
pub const PREVIEW_STAR_BRIGHTNESS: f32 = 0.32;
/// Deriva angular relativa del cielo (radianes/segundo de animacion).
/// No modifica la metrica ni pretende simular un agujero en traslacion.
pub const SKY_DRIFT_SPEED: f32 = 0.012;

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
/// Fraccion de distancia por click de rueda, como exponente.
///
/// Un click multiplica la distancia por `exp(0.05)`, o sea un 5%. Estaba en 0.12
/// y saltaba.
pub const ZOOM_SENSITIVITY: f32 = 0.05;
/// Tope de clicks de rueda que se atienden en un solo frame.
///
/// Algunos drivers reportan el scroll acumulado en vez de un click por evento, y
/// un golpe de rueda rapido puede llegar como un delta enorme en un unico frame.
/// Sin tope, ese frame se come todo el rango de zoom de una.
pub const ZOOM_MAX_STEP: f32 = 2.5;
pub const KEY_ORBIT_SPEED: f32 = 1.1;
pub const KEY_ZOOM_SPEED: f32 = 5.0;
/// Tope del tiempo de frame que se usa para el zoom por teclado.
///
/// Mas bajo que `MAX_FRAME_DELTA` a proposito. El primer frame despues de empezar
/// a mover la camara todavia llega con la duracion del framerate lento, porque la
/// resolucion recien baja en el frame siguiente: sin este tope, cada zoom arranca
/// con un salto y despues se suaviza.
pub const ZOOM_MAX_FRAME_DELTA: f32 = 0.05;
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
pub const PREVIEW_BLOOM_INTENSITY: f32 = 0.24;
pub const HDR_CEILING: f32 = 150.0;
pub const EXPOSURE: f32 = 5.0;
pub const GAMMA: f32 = 2.2;
pub const NOISE_TABLE_SIZE: usize = 256;
pub const NOISE_SEED: u32 = 0x9E37_79B9;

// --- Version Endurance -------------------------------------------------------
// Escala cinematografica: la nave real mide decenas de metros y Gargantua
// millones de kilometros. Aca el anillo mide dos centesimas de rs: la nave es
// diminuta frente al agujero, como en la pelicula, y el zoom permite acercarse
// hasta ver sus detalles.

/// Posicion de la nave: radio horizontal, altura sobre el disco y azimut.
/// A 0.3 rs del plano vuela rozando la bruma del gas.
pub const SHIP_ORBIT_RADIUS: f32 = 7.6;
pub const SHIP_HEIGHT: f32 = 0.3;
pub const SHIP_AZIMUTH: f32 = 0.6;
/// Radio del anillo de modulos, en rs.
pub const SHIP_SCALE: f32 = 0.02;
/// Rumbo: angulo entre la direccion de vuelo y la que apunta al agujero,
/// hacia el sentido de giro del disco. Cabeceo y alabeo de la nave.
pub const SHIP_HEADING: f32 = 0.5;
pub const SHIP_PITCH: f32 = 0.0;
pub const SHIP_BANK: f32 = 0.12;
/// Giro del anillo con `G`, en radianes por segundo.
pub const SHIP_SPIN_SPEED: f32 = 0.35;
pub const SHIP_MARCH_STEPS: usize = 160;
/// Rebotes de reflexion y refraccion por rayo de camara.
pub const SHIP_MAX_BOUNCES: u32 = 3;
/// Direcciones con que se captura la luz del entorno en la nave.
pub const SHIP_LIGHT_SAMPLES: usize = 2048;
/// Absorcion del vidrio por unidad de longitud local (Beer-Lambert), RGB.
pub const GLASS_ABSORPTION: Vec3 = Vec3::new(0.9, 0.35, 0.25);

/// Camara de la Endurance: orbita la nave. El yaw se mide desde la posicion
/// detras de la nave, mirando en su direccion de vuelo.
pub const ENDURANCE_CAMERA_DISTANCE: f32 = 1.7;
pub const ENDURANCE_CAMERA_YAW: f32 = 0.0;
pub const ENDURANCE_CAMERA_PITCH: f32 = 0.06;
pub const ENDURANCE_CAMERA_MIN_DISTANCE: f32 = 0.045;
pub const ENDURANCE_CAMERA_MAX_DISTANCE: f32 = 12.0;
/// Altura minima de la camara: por debajo entraria al gas opaco.
pub const ENDURANCE_CAMERA_FLOOR: f32 = 0.08;
/// Radio alrededor del agujero que la camara no cruza al alejarse de la nave.
pub const ENDURANCE_CAMERA_KEEP_OUT: f32 = 2.5;
/// Zoom mas rapido que alrededor del agujero: el rango es de mas de 260 veces.
pub const ENDURANCE_ZOOM_SENSITIVITY: f32 = 0.11;

/// Skybox: cubemap de seis caras generado al iniciar la version.
pub const SKYBOX_FACE_SIZE: usize = 768;
pub const SKYBOX_BRIGHTNESS: f32 = 0.012;
/// Brillo del skybox en cada version: la variante expone mas que la Endurance.
pub const VARIANT_SKY_GAIN: f32 = 1.1;
pub const ENDURANCE_SKY_GAIN: f32 = 1.0;
/// Polo y centro galacticos: orientan la banda de la Via Lactea.
pub const GALACTIC_POLE: Vec3 = Vec3::new(0.34, 0.94, -0.06);
pub const GALACTIC_CENTER: Vec3 = Vec3::new(-0.75, 0.23, -0.62);

/// Temperatura de color de pico del disco en la Endurance: mas calida que la
/// variante, como el Gargantua dorado de la pelicula. El brillo no cambia.
pub const ENDURANCE_DISK_TEMPERATURE: f32 = 5200.0;
/// Alturas de la atmosfera y de la envoltura gris en la Endurance: un mar de
/// nubes mas compacto que el de la variante.
pub const ENDURANCE_ATMOSPHERE_HEIGHT_RATIO: f32 = 0.012;
pub const ENDURANCE_OUTER_GAS_HEIGHT_RATIO: f32 = 0.014;

/// Presentacion cinematografica de la Endurance.
pub const ENDURANCE_EXPOSURE: f32 = 1.3;
pub const ENDURANCE_BLOOM_INTENSITY: f32 = 0.2;
pub const STREAK_INTENSITY: f32 = 0.1;
pub const STREAK_THRESHOLD: f32 = 12.0;
pub const STREAK_LENGTH: f32 = 0.12;
/// Velo de la lente: halo muy ancho y tenue alrededor del disco.
pub const GLARE_INTENSITY: f32 = 0.15;
pub const FILM_VIGNETTE: f32 = 0.38;
pub const FILM_GRAIN: f32 = 0.018;
/// Saturacion despues de la curva: 1 conserva el color del cuerpo negro.
pub const FILM_SATURATION: f32 = 0.72;
/// Relacion de aspecto de las barras de cine (anamorfico 2.39:1).
pub const LETTERBOX_ASPECT: f32 = 2.39;
