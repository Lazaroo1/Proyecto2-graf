# Proyecto 2 — Agujero negro de Schwarzschild

Gráficas por Computadora · Lázaro Díaz, 24713

Render en CPU de un agujero negro y su disco de acreción. La silueta y las
imágenes múltiples salen de integrar geodésicas de Schwarzschild. El gas tiene
un núcleo delgado, una atmósfera tenue, rotación diferencial y transferencia
de emisión/absorción. El color incorpora Doppler y corrimiento gravitacional.

Rust, sin GPU. Dependencias: `minifb`, `rayon`, `glam`.

## Ejecutar

```sh
cargo run --release
```

La primera vista se refina durante dos frames: se calculan las trayectorias
subpíxel y luego se reutilizan. El plasma se sigue calculando en cada frame.
Mover la cámara invalida las trayectorias; no se reutiliza una imagen vieja.

| Acción | Control |
| --- | --- |
| Orbitar | Arrastrar o W/A/S/D |
| Acercar / alejar | Rueda o flechas arriba/abajo |
| Vista de canto, estilo cinematográfico | 1 |
| Vista inclinada a 30° | 2 |
| Vista casi polar, 87° | 3 |
| Pausar / continuar el gas | Espacio |
| Restablecer cámara | R |
| Salir | Esc |

El título de la ventana muestra los FPS y el estado de pausa.

## Capturas y verificación

```sh
cargo test --release
cargo clippy --all-targets -- -D warnings
cargo run --release -- --verify
cargo run --release -- --probe 30 artifacts/frame.ppm
cargo run --release -- --probe 20 artifacts/inclinada.ppm 30 36 300
```

`--probe frames salida.ppm [elevación] [distancia] [tiempo_inicial]` mide
arrastre real y cámara quieta. Informa tanto el promedio con preparación como
el tiempo por frame después de preparar la lente. La animación avanza a 30
muestras por segundo de tiempo simulado.

Para exportar una secuencia a 30 FPS, sin ventana:

```sh
cargo run --release -- --sequence artifacts/secuencia 90 2 21.5 0 960 540
```

Argumentos: directorio, frames, elevación, distancia, tiempo inicial, ancho,
alto. Un solo frame permite exportar una captura a otra resolución:

```sh
cargo run --release -- --sequence artifacts/captura 1 30 36 2 1920 1080
```

Los archivos PPM contienen el render sin compresión. Las capturas de revisión
de esta sesión están en `artifacts/`:

- [Antes / después y distintas vistas](artifacts/comparison.png)
- [Vista cinematográfica](artifacts/cinematic.png)
- [Movimiento a 30 FPS de reproducción](artifacts/blackhole-motion.gif)
- [Vista inclinada a los 300 segundos](artifacts/late.png)

El GIF es una exportación, no una medición de FPS en vivo. El respaldo previo
a los cambios está en `artifacts/before-changes.zip`.

## Errores corregidos

1. **Ruido 3D discontinuo.** Los dos canales de la tabla eran independientes.
   El final de un slice Z no coincidía con el inicio del siguiente. El segundo
   canal ahora referencia exactamente el slice desplazado `(37,17)`.
2. **Reinicio visible del flujo.** La capa renovada tenía peso máximo justo al
   cambiar de identidad. Ahora nace y desaparece con peso y derivada cero.
   Las identidades quedan acotadas para evitar pérdida de precisión.
3. **Giro contrario al Doppler.** `atan2(z,x)` decrece para rotación positiva
   alrededor de Y. La advección usa ahora el mismo sentido que el momento
   angular del transporte de luz.
4. **Historia temporal excesiva.** Un peso fijo de 0.88 mezclaba más de un
   segundo de movimiento al funcionar a pocos FPS. La constante temporal es
   ahora de 45 ms y el historial se descarta al cambiar la vista.
5. **Superficie sin espesor.** Se sustituyó el plano por densidad gaussiana
   vertical y una atmósfera de baja profundidad óptica. El muestreo adaptativo
   resuelve el núcleo sin colapsar el paso sobre `y=0`.
6. **Color y detalle perdidos.** El tonemap anterior comprimía cada canal,
   añadía contraste y ganancia hasta saturar las zonas calientes. El nuevo
   operador aplica un factor común a RGB y conserva su cromaticidad.
7. **Marco de la cámara.** La dirección medida por el observador se transforma
   de su marco local a coordenadas de Schwarzschild antes de integrar.
8. **Costo repetido.** Se separaron geometría y emisión: cache de geodésicas
   por vista, campo de plasma compartido y regenerado por frame, muestras
   espaciales reutilizadas y reescalado bilineal.
9. **Bloom al cambiar resolución.** El tamaño del halo se fija en píxeles de
   ventana para que no se ensanche al arrastrar.
10. **Verificación que no fallaba el proceso.** `--verify` ahora devuelve
    código distinto de cero si falla una comprobación.

## Modelo físico y límites

Unidades: `rs = c = 1`, `M = 1/2`. Horizonte en 1, esfera de fotones en
1.5, ISCO en 3 y parámetro de impacto crítico `3√3 M ≈ 2.598076`.

La trayectoria satisface:

```text
d²u/dφ² + u = 3 M u²
a = -(3/2) rs |r × v|² r / |r|⁵
```

Se integra con RK4 y límites de paso por curvatura, radio y espesor del gas.
La sombra, el arco superior, el inferior y el anillo fino no son primitivas
dibujadas en pantalla.

Para órbitas circulares ecuatoriales, con eje del disco Y:

```text
Ω = √(M/r³)
g = √(1 - 3M/r) / [(1 - Ω L_y/E) √(1 - rs/r_observador)]
T_observada = g T_emitida
I_bolométrica_observada = g⁴ I_bolométrica_emitida
```

El invariante espectral es **Iν/ν³**; integrar en frecuencia aporta el cuarto
factor de g. Se aplica una sola vez. El lado que se acerca se ve más brillante
y azulado; el que se aleja, más tenue y cálido. Desde el eje del disco el
Doppler longitudinal se anula, por lo que una vista polar no debe tener una
media luna fija pintada de un lado.

La absorción por segmento usa `α = 1 - exp(-τ)`, composición de adelante hacia
atrás y longitud comóvil `dℓ = dλ/g`, para energía local inicial unitaria.

Las aproximaciones están separadas de las geodésicas:

- El agujero es **Schwarzschild**, no Kerr: no hay rotación del agujero ni
  arrastre de marcos. El plasma sí orbita. No es una reproducción exacta del
  Gargantua rotante de la película.
- Temperatura con perfil newtoniano de disco delgado y torque nulo en la ISCO,
  no el perfil completo relativista de Novikov–Thorne.
- Núcleo gaussiano con H/r = 0.004 y atmósfera tenue con H/r = 0.035.
  Se extrapola la velocidad ecuatorial a ese pequeño espesor.
- Turbulencia, fluctuaciones de emisión y densidad procedurales: no se
  resuelven magnetohidrodinámica, equilibrio vertical ni acreción.
- Color por aproximación del lugar planckiano y brillo bolométrico T⁴;
  no se integra la respuesta espectral de una cámara real.
- Escena instantánea: sin retardos de propagación entre distintos caminos,
  dispersión Compton, polarización ni retroacción del gas sobre la métrica.
- Exposición y bloom modelan la presentación de la cámara. No alteran
  geodésicas ni frecuencias.

La quinta referencia del encargo sirve como referencia de morfología;
este modelo térmico óptico no reproduce las observaciones de radio del EHT.

Referencias: [visualización de NASA](https://www.nasa.gov/universe/nasa-visualization-shows-a-black-holes-warped-world/)
y [James et al., lente gravitacional en Interstellar](https://arxiv.org/abs/1502.03808).

## Resultados de esta revisión

En esta máquina, 960×540:

| Medida | Antes | Ahora |
| --- | --- | --- |
| Cámara quieta, animación sostenida | ~5.6 FPS | ~17 FPS tras preparar la lente |
| Arrastre, escala 0.33 | ~33 FPS en probe previo sin mover cámara | ~16 FPS con movimiento real |
| Píxeles con luminancia ≥ 250 en captura de canto | ~5.14 % | 0 % |

La comparación de arrastre no es directa: el probe viejo mantenía la cámara
quieta. La preparación inicial de las trayectorias cuesta más que un frame
sostenido y se repite al cambiar la vista. Los números varían con el CPU,
la temperatura y la inclinación.

12 pruebas automatizadas comprueban continuidad espacial/temporal, sentido
de advección, Doppler contra el marco local, profundidad óptica, cache,
tonemap y los cuatro resultados analíticos de Schwarzschild. En la corrida
final: deriva de momento angular/energía ~1e-6 y error de deflexión ~0.425 %
frente a la serie de dos términos.

## Archivos principales

| Archivo | Responsabilidad |
| --- | --- |
| `src/main.rs` | Ventana, pausa, benchmark y exportación |
| `src/config.rs` | Constantes de física, gas y cámara |
| `src/camera.rs`, `src/input.rs` | Proyección, órbita y vistas |
| `src/scene/relativity.rs` | Geodésicas, frecuencias y perfil térmico |
| `src/scene/blackhole.rs` | Inicialización local y RK4 |
| `src/scene/disk.rs` | Medio emisor, absorción y textura del gas |
| `src/math/noise.rs` | Ruido continuo y fBm |
| `src/render/raymarch.rs` | Integración volumétrica y cache de trayectorias |
| `src/render/mod.rs` | Orquestación del pipeline |
| `src/render/accumulate.rs` | Historial corto en tiempo real |
| `src/render/bloom.rs`, `src/render/tonemap.rs` | Halo y compresión HDR |
| `src/render/framebuffer.rs` | Reescalado y presentación |
| `src/verify.rs` | Comprobaciones analíticas |

`CODIGO-COMPLETO.md` contiene el volcado actualizado de fuentes y documentación.
