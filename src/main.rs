//! Proyecto 2 - Graficas por Computadora
//!
//! Animacion de un agujero negro por raymarching en CPU: disco de acrecion
//! giratorio, geodesicas de Schwarzschild y post-proceso con bloom y
//! tonemapping.
//!
//! Este archivo es solo el loop: abre la ventana, lleva el tiempo, lee el input
//! y le pide un frame al `Renderer`. Toda la matematica vive en `math/`, la
//! escena en `scene/` y el pipeline en `render/`.

mod camera;
mod config;
mod input;
mod math;
mod render;
mod scene;
mod verify;

use std::time::Instant;

use minifb::{Key, Window, WindowOptions};

use camera::OrbitCamera;
use input::InputState;
use render::Renderer;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let enhanced = !args.iter().any(|arg| arg == "--original");

    if args.iter().any(|a| a == "--verify") {
        if !verify::run() {
            std::process::exit(1);
        }
        return;
    }

    if args.first().is_some_and(|a| a == "--sequence") {
        export_sequence(&args[1..], enhanced);
        return;
    }

    if let Some(index) = args.iter().position(|a| a == "--probe") {
        let frames = args
            .get(index + 1)
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(10)
            .max(1);
        let dump = args.get(index + 2).cloned();
        let pitch = args.get(index + 3).and_then(|s| s.parse::<f32>().ok());
        let distance = args.get(index + 4).and_then(|s| s.parse::<f32>().ok());
        let start = args.get(index + 5).and_then(|s| s.parse::<f32>().ok());
        probe(frames, dump, pitch, distance, start, enhanced);
        return;
    }

    run(enhanced);
}

/// Loop interactivo: ventana, input y presentacion.
fn run(enhanced: bool) {
    let mut window = Window::new(
        config::WINDOW_TITLE,
        config::WINDOW_WIDTH,
        config::WINDOW_HEIGHT,
        WindowOptions::default(),
    )
    .expect("no se pudo abrir la ventana");

    // minifb duerme lo que sobre de cada frame. Sin esto el loop consume el
    // 100% de un nucleo aunque el render sea trivial.
    window.set_target_fps(config::TARGET_FPS);

    let mut camera = OrbitCamera::new();
    let mut input = InputState::new();
    let mut renderer = Renderer::new(config::WINDOW_WIDTH, config::WINDOW_HEIGHT);
    renderer.set_enhanced(enhanced);

    let mut time = 0.0;
    let mut paused = false;
    let mut previous_frame = Instant::now();
    let mut title_updated = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Duracion del frame anterior, para que el teclado gire a velocidad
        // constante. Se acota por arriba: si la ventana estuvo minimizada o el
        // proceso se colgo un segundo, el primer frame de vuelta daria un salto
        // de camara enorme.
        let elapsed = previous_frame.elapsed().as_secs_f32();
        let dt = elapsed.min(config::MAX_FRAME_DELTA);
        previous_frame = Instant::now();

        input.poll(&window, dt);
        input.apply_to(&mut camera);
        if window.is_key_pressed(Key::V, minifb::KeyRepeat::No) {
            renderer.set_enhanced(!renderer.enhanced());
        }

        if window.is_key_pressed(Key::Space, minifb::KeyRepeat::No) {
            paused = !paused;
        }
        if !paused {
            time += elapsed;
        }
        if title_updated.elapsed().as_secs_f32() > 0.5 {
            window.set_title(&format!(
                "{} | {:.0} FPS | V: {} | 1 cine 2 inclinada 3 arriba | Espacio: {}",
                config::WINDOW_TITLE,
                1.0 / elapsed.max(0.001),
                if renderer.enhanced() {
                    "variante"
                } else {
                    "original"
                },
                if paused { "continuar" } else { "pausa" }
            ));
            title_updated = Instant::now();
        }

        let frame = renderer.render(&camera, time, input.camera_moved());

        window
            .update_with_buffer(frame, config::WINDOW_WIDTH, config::WINDOW_HEIGHT)
            .expect("no se pudo presentar el frame");
    }
}

/// Secuencia reproducible sin ventana. Se puede convertir a GIF/video fuera
/// del renderer sin agregar dependencias de codificacion al proyecto.
fn export_sequence(args: &[String], enhanced: bool) {
    let Some(directory) = args.first() else {
        eprintln!(
            "Uso: --sequence directorio [frames] [elevacion] [distancia] [tiempo] [ancho] [alto]"
        );
        std::process::exit(2);
    };
    let number = |i: usize, default: f32| {
        args.get(i)
            .and_then(|s| s.parse::<f32>().ok())
            .filter(|x| x.is_finite())
            .unwrap_or(default)
    };
    let frames = number(1, 90.0).clamp(1.0, 1800.0) as usize;
    let mut camera = OrbitCamera::new();
    camera.pitch = number(2, config::CAMERA_PITCH.to_degrees())
        .to_radians()
        .clamp(-config::CAMERA_PITCH_LIMIT, config::CAMERA_PITCH_LIMIT);
    camera.distance = number(3, config::CAMERA_DISTANCE)
        .clamp(config::CAMERA_MIN_DISTANCE, config::CAMERA_MAX_DISTANCE);
    let start = number(4, 0.0);
    let width = number(5, 960.0).clamp(32.0, 3840.0) as usize;
    let height = number(6, 540.0).clamp(32.0, 2160.0) as usize;
    std::fs::create_dir_all(directory).expect("no se pudo crear directorio de salida");
    let mut renderer = Renderer::new(width, height);
    renderer.set_enhanced(enhanced);
    // Preparar antialias antes del primer frame exportado.
    for _ in 0..config::SPATIAL_SAMPLES {
        renderer.render(&camera, start, false);
    }
    for i in 0..frames {
        let frame = renderer.render(&camera, start + i as f32 / 30.0, false);
        let path = std::path::Path::new(directory).join(format!("frame-{i:04}.ppm"));
        write_ppm(path.to_str().expect("ruta UTF-8"), frame, width, height);
    }
}

/// Render sin ventana, para medir y para tunear.
///
/// Corre `frames` frames en cada uno de los dos modos de resolucion (quieto y
/// arrastrando) e imprime, para cada uno, el tiempo por frame y la distribucion
/// de brillo del ultimo. Con un tercer argumento, guarda ese frame como PPM.
///
/// El cuarto y quinto argumento son la elevacion de la camara en grados y su
/// distancia. Sirven para revisar poses que no son la de arranque sin tener que
/// arrastrar el mouse: el disco visto de frente, por ejemplo, revela defectos que
/// de canto quedan escondidos, porque ahi la estructura del gas se ve entera en
/// vez de comprimida en una linea.
///
/// El sexto es el instante inicial, en segundos. Sin el, el probe siempre mide
/// los primeros 0.4 segundos de animacion, y hay defectos que solo aparecen mucho
/// despues: el patron del gas se va deformando con el tiempo, asi que un render
/// en `t = 0` puede verse perfecto mientras que el mismo render en `t = 60` esta
/// arruinado. Comparar dos instantes separados tambien es la unica forma de medir
/// si la escena de verdad se esta moviendo.
///
/// La distribucion es lo util para tunear: si la media esta cerca de 0 la escena
/// quedo subexpuesta, si esta cerca de 255 se quemo, y el porcentaje de pixeles
/// saturados dice cuanto se esta perdiendo en el recorte. Ajustar `EXPOSURE`,
/// `DISK_BRIGHTNESS` y `BLOOM_INTENSITY` a ojo contra una ventana es mucho mas lento
/// que mirar esos tres numeros.
fn probe(
    frames: usize,
    dump: Option<String>,
    pitch: Option<f32>,
    distance: Option<f32>,
    start: Option<f32>,
    enhanced: bool,
) {
    let start = start.unwrap_or(0.0);
    let (width, height) = (config::WINDOW_WIDTH, config::WINDOW_HEIGHT);
    let mut camera = OrbitCamera::new();
    if let Some(degrees) = pitch {
        camera.pitch = degrees.to_radians();
    }
    if let Some(radius) = distance {
        camera.distance = radius;
    }

    println!(
        "probe: {frames} frames a {width}x{height}, elevacion {:.1} grados, distancia {:.1} rs, t = {start:.1} s",
        camera.pitch.to_degrees(),
        camera.distance
    );

    // El modo barato va primero. Al reves, los minutos de carga a resolucion
    // completa calientan el CPU lo suficiente como para que empiece a bajar de
    // frecuencia, y la segunda medicion sale hasta el doble de lenta sin que
    // nada del codigo haya cambiado.
    for (label, moved, scale) in [
        ("drag", true, config::RENDER_SCALE_DRAG),
        ("idle", false, config::RENDER_SCALE_IDLE),
    ] {
        // Un renderer nuevo por modo: compartirlo arrastraria el historico de
        // acumulacion de un modo al otro y el primer frame saldria mezclado.
        let mut renderer = Renderer::new(width, height);
        renderer.set_enhanced(enhanced);
        let mut total = std::time::Duration::ZERO;
        let mut warm_total = std::time::Duration::ZERO;
        let mut warm_frames = 0;

        for i in 0..frames {
            let started = Instant::now();
            // Medir movimiento real de camara: si no cambia la vista,
            // el cache tambien acelera el modo reducido y falsea el benchmark.
            if moved {
                camera.yaw += 0.012;
            }
            let buffer = renderer.render(&camera, start + i as f32 / 30.0, moved);
            let elapsed = started.elapsed();
            total += elapsed;
            if i >= config::SPATIAL_SAMPLES {
                warm_total += elapsed;
                warm_frames += 1;
            }

            if i + 1 == frames {
                report_histogram(buffer);
                if !moved {
                    if let Some(path) = dump.as_deref() {
                        write_ppm(path, buffer, width, height);
                    }
                }
            }
        }

        let per_frame = total.as_secs_f64() / frames as f64;
        println!(
            "  {label}  escala {scale:.2}  {:.1} ms/frame  ({:.1} fps)",
            per_frame * 1000.0,
            1.0 / per_frame
        );
        if warm_frames > 0 {
            let warm = warm_total.as_secs_f64() / warm_frames as f64;
            println!(
                "  {label}  tras preparar lente: {:.1} ms/frame ({:.1} fps)",
                warm * 1000.0,
                1.0 / warm
            );
        }
        // Restituir yaw para que ambas mediciones tengan la pose solicitada.
        if moved {
            camera.yaw -= frames as f32 * 0.012;
        }
    }
}

/// Resume la distribucion de brillo del frame.
fn report_histogram(buffer: &[u32]) {
    let mut min = 255u32;
    let mut max = 0u32;
    let mut sum = 0f64;
    let mut saturated = 0usize;

    for &packed in buffer {
        let r = (packed >> 16) & 0xFF;
        let g = (packed >> 8) & 0xFF;
        let b = packed & 0xFF;
        // Luma aproximada con pesos 2:5:1, que es Rec. 709 redondeado a enteros.
        let luma = (r * 2 + g * 5 + b) / 8;
        min = min.min(luma);
        max = max.max(luma);
        sum += luma as f64;
        if luma >= 250 {
            saturated += 1;
        }
    }

    println!(
        "  luma  min {min}  max {max}  media {:.1}  saturados {:.2}%",
        sum / buffer.len() as f64,
        100.0 * saturated as f64 / buffer.len() as f64
    );
}

/// Guarda el frame como PPM binario (P6).
///
/// Sin dependencias: PPM es una cabecera de texto seguida de bytes RGB crudos.
/// Cualquier visor o `ffmpeg` lo abre, y sirve para sacar capturas para el
/// informe sin tener que fotografiar la ventana.
fn write_ppm(path: &str, buffer: &[u32], width: usize, height: usize) {
    let mut out = Vec::with_capacity(width * height * 3 + 32);
    out.extend_from_slice(
        format!(
            "P6
{width} {height}
255
"
        )
        .as_bytes(),
    );
    for &packed in buffer {
        out.push((packed >> 16) as u8);
        out.push((packed >> 8) as u8);
        out.push(packed as u8);
    }
    match std::fs::write(path, out) {
        Ok(()) => println!("  frame guardado en {path}"),
        Err(e) => eprintln!("  no se pudo escribir {path}: {e}"),
    }
}
