# Cómo funciona el proyecto Gargantua

Guía para entender y explicar el proyecto: qué hace cada archivo, cuáles son
las ecuaciones que lo sostienen y cómo se forma un agujero negro de verdad, con
analogías de su tamaño, incluido TON 618, uno de los más masivos conocidos. Al
final hay un glosario con los términos técnicos.

## Cómo encaja todo

```text
main.rs ──► Renderer (render/mod.rs)
              │
              ├─ camera.rs        un rayo por píxel
              ├─ raymarch.rs      sigue cada rayo como luz curvada
              │    ├─ blackhole.rs + relativity.rs   la curva de la luz (física)
              │    ├─ disk.rs                         el gas que brilla
              │    ├─ endurance.rs + material.rs      la nave y sus materiales
              │    ├─ lighting.rs                     la luz que recibe la nave
              │    └─ skybox.rs + stars.rs            el fondo
              ├─ accumulate.rs    suaviza con el frame anterior
              ├─ bloom.rs         halo, velo de lente y destellos
              ├─ tonemap.rs       de luz HDR a colores de pantalla
              └─ framebuffer.rs   imagen final para la ventana (minifb)
```

## 1. Los archivos, del más importante al menos importante

El orden sigue un criterio simple: cuánto depende la imagen final de ese
archivo. Los primeros son el núcleo del render; los últimos, soporte.

1. **`src/render/raymarch.rs` — el corazón del render.** Lanza un rayo por
   píxel y lo sigue paso a paso como una geodésica, es decir, como luz curvada
   por la gravedad. En cada paso revisa si hay gas (y guarda una muestra del
   disco) y, en la Endurance, si ese tramo del rayo toca la nave. Si la toca,
   sombrea el punto y lanza rayos hijos de reflexión y refracción, que también
   se curvan. Todo queda en una caché por vista: con la cámara quieta, cada
   frame solo vuelve a pintar las muestras con el gas del instante actual, por
   eso el disco y sus reflejos siguen animados sin recalcular trayectorias.
   También captura la luz del entorno de la nave. Funciones clave:
   `RayCache::render`, `march`, `shade_hit`, `capture_environment`.

2. **`src/scene/blackhole.rs` — el fotón.** Define `Photon`, el estado de un
   rayo de luz: posición y velocidad. `from_camera` convierte la dirección del
   píxel al marco del agujero, `advance` da un paso con Runge–Kutta de cuarto
   orden, `step_length` elige el tamaño del paso según cuánto se curva la luz,
   `captured` detecta si el rayo cayó y `escaped`, si se fue lejos. Los rayos
   se trazan al revés: de la cámara hacia la fuente de luz.

3. **`src/scene/relativity.rs` — la física de Schwarzschild.** Contiene la
   aceleración de la geodésica, la energía y el momento angular que se
   conservan, la velocidad orbital del gas, el factor de corrimiento `g`
   (Doppler más gravedad), el corrimiento al azul del cielo y el perfil de
   temperatura del disco. Es el único archivo con relatividad general pura; los
   demás lo usan.

4. **`src/scene/disk.rs` — el disco de acreción.** Para cada punto del espacio
   calcula cuánto gas hay (perfiles en altura: núcleo denso, atmósfera y
   envoltura gris), qué color y brillo emite según su temperatura y su
   corrimiento `g`, y cuánta luz absorbe. `Medium` guarda la forma del gas de
   cada versión. `GasTexture` genera la turbulencia animada: ruido fractal
   deformado que gira con la velocidad orbital de cada radio (más rápido
   adentro), mezclando dos campos para que nunca se note un reinicio.

5. **`src/render/mod.rs` — el director de orquesta.** El `Renderer` arma cada
   frame en orden: actualiza el gas, genera el skybox, la luz y la nave si hacen
   falta, traza o reutiliza los rayos, mezcla con el frame anterior, aplica el
   bloom (y en la Endurance el velo de lente y los destellos) y por último el
   tonemap. Mientras la cámara se mueve, baja la resolución a un tercio para
   mantener la fluidez.

6. **`src/main.rs` — el punto de entrada.** Abre la ventana con `minifb`, lleva
   el tiempo y corre el bucle principal: lee teclado y mouse, maneja `V`
   (versión), `G` (giro del anillo), `B` (barras de cine) y `Espacio` (pausa), y
   pide un frame al `Renderer`. También tiene modos sin ventana: `--verify`
   (física), `--probe` (FPS y capturas), `--sequence` (exportar frames) y
   `--materials` (muestras de texturas).

7. **`src/camera.rs` — la cámara.** Cámara orbital en coordenadas esféricas
   (yaw, pitch y distancia) que siempre mira a su objetivo: el agujero o la
   nave. Genera la dirección del rayo de cada píxel (`ray_for_pixel`). En la
   Endurance tiene zoom de 0.045 a 12 rs, un piso que no la deja entrar al gas y
   una esfera de exclusión que no la deja caer al agujero. Define las vistas 1,
   2 y 3 de cada versión.

8. **`src/scene/endurance.rs` — la nave.** Describe la Endurance con funciones
   de distancia con signo (SDF): 12 módulos de tres tipos, túneles, radios
   dobles, núcleo, cúpula de vidrio y el transbordador Ranger. `intersect`
   encuentra el impacto con sphere tracing. `shade` calcula el color del punto:
   normal, textura, relieve, Fresnel, luz difusa, sombras suaves, brillos GGX y
   reflejo borroso. `through_glass` sigue la luz dentro de la cúpula: Snell,
   reflexión total interna y absorción del vidrio.

9. **`src/scene/material.rs` — los seis materiales.** Tabla con el albedo,
   specular, rugosidad, transparencia, reflectividad, índice de refracción,
   metalicidad y emisión de cada material, y su textura procedural: paneles del
   casco, lámina dorada, celdas solares, vidrio con nervios, losetas térmicas y
   metal quemado. Las texturas reducen su detalle según el tamaño del píxel para
   no parpadear de lejos. También contiene la óptica: `reflect`, `refract`,
   `fresnel` y `ggx`.

10. **`src/scene/lighting.rs` — la luz que recibe la nave.** Recibe 2048
    muestras de luz capturadas desde la posición de la nave y las resume: las
    tres zonas más brillantes se vuelven luces clave, con sombra, y el resto se
    guarda en armónicos esféricos, que dan la luz difusa para cualquier
    orientación con solo 9 números por color.

11. **`src/scene/skybox.rs` — el fondo.** Al iniciar genera un cubemap de seis
    caras de 768×768 píxeles con la Vía Láctea (banda, bulbo, franjas de polvo,
    regiones rosadas y nebulosas). Después solo lo consulta, con interpolación
    bilineal, en la dirección de salida de cada rayo. Guarda los colores en
    formato RGBE: luz HDR en 4 bytes por píxel.

12. **`src/scene/stars.rs` — las estrellas.** Genera estrellas puntuales sin
    imágenes: divide el cielo en celdas sobre las caras de un cubo y un hash
    decide si hay estrella, dónde, de qué temperatura (color) y con qué brillo.
    `SkyFrame` hace derivar el cielo lentamente y aplica el corrimiento al azul
    que ve un observador cerca del agujero.

13. **`src/config.rs` — todos los parámetros.** Constantes agrupadas: ventana,
    física (rs = 1 y radios característicos), precisión de la integración,
    disco, gas, estrellas, cámara, bloom, exposición y todo lo de la Endurance
    (posición y tamaño de la nave, cámara, skybox y efectos de cine). Cambiar el
    aspecto del render es, sobre todo, cambiar números aquí.

14. **`src/render/tonemap.rs` — de luz HDR a pantalla.** El render trabaja con
    luz sin tope (HDR) y este archivo la comprime al rango que muestra un
    monitor. En la original y la variante usa una curva que conserva el color y
    gamma 2.2. En la Endurance usa la curva de película ACES, baja la
    saturación y agrega viraje de color, viñeta, grano y barras 2.39:1.

15. **`src/render/bloom.rs` — el halo.** Extrae lo más brillante de la imagen,
    lo reduce en seis niveles y lo desenfoca con un filtro gaussiano; al
    sumarlos aparece el resplandor alrededor del disco. En la Endurance agrega
    el velo de lente (los niveles más anchos) y los destellos horizontales de
    lente anamórfica.

16. **`src/parallel.rs` — los hilos.** Reparte las filas de la imagen entre
    todos los núcleos del CPU con hilos persistentes de la biblioteca estándar
    de Rust. Sin esto el render sería varias veces más lento.

17. **`src/math/blackbody.rs` — de temperatura a color.** Convierte una
    temperatura en kelvin al color de un cuerpo negro, con una aproximación del
    lugar planckiano (CIE) y la matriz de XYZ a sRGB. Así el color del disco
    sale de la física: más caliente es más blanco azulado y más frío, más
    naranja.

18. **`src/math/noise.rs` — el ruido.** Ruido de valor 3D desde una tabla
    precomputada y fBm (suma de octavas de ruido). Es la base de la turbulencia
    del gas, del skybox y de varias texturas.

19. **`src/math/vector.rs` — el álgebra.** `Vec2`, `Vec3` y `Mat3` propios:
    suma, producto punto y cruz, normalización, interpolación, rotaciones y
    transpuesta. Todo el proyecto los usa.

20. **`src/render/framebuffer.rs` — los buffers.** Guarda la imagen HDR y al
    final la reescala con interpolación bilineal al tamaño de la ventana,
    empaquetándola en el formato `0x00RRGGBB` que pide `minifb`.

21. **`src/input.rs` — los controles.** Lee el mouse (arrastrar para orbitar,
    rueda para el zoom) y el teclado (WASD, flechas, 1, 2, 3 y R) y los
    convierte en movimientos de cámara que no dependen de los FPS.

22. **`src/version.rs` — las tres versiones.** El tipo `Version` (original,
    variante y Endurance), el orden en que `V` las recorre y el argumento de
    línea de comandos que elige cada una.

23. **`src/render/accumulate.rs` — suavizado temporal.** Mezcla el frame actual
    con el anterior, hasta un 72 %, para reducir ruido sin dejar estelas
    largas. Se desactiva al mover la cámara.

24. **`src/verify.rs` — la física comprobada.** `--verify` compara el
    integrador con resultados exactos de la relatividad: el tamaño de la sombra,
    la conservación de energía y momento, la órbita circular de la luz en
    r = 1.5 rs y la desviación de la luz de Einstein.

25. **`src/math/curves.rs`** — `saturate`, `smoothstep` y `remap`:
    transiciones suaves que se usan en todo el proyecto.

26. **`src/math/sdf.rs`** — distancia a una esfera; detecta cuándo un rayo
    cruza el horizonte de eventos.

27. **`src/math/ray.rs`** — el tipo `Ray`: origen y dirección normalizada.

28. **`Cargo.toml`** — el manifiesto: nombre del proyecto, la única dependencia
    (`minifb`) y los perfiles de compilación optimizados.

29. **`README.md`** — la documentación completa: versiones, controles,
    rúbrica, ecuaciones, validación y rendimiento.

30. **`src/math/mod.rs`** — declara los módulos de matemática y reexporta
    `Vec2`, `Vec3`, `Mat3` y `Ray`.

31. **`src/scene/mod.rs`** — declara los módulos de la escena.

32. **`Cargo.lock`** — versiones exactas de las dependencias internas de
    `minifb`, para que el proyecto compile igual en cualquier máquina.

33. **`.gitignore`** — excluye de git la carpeta `target/`, las exportaciones de
    `artifacts/` y las capturas `.ppm` y `.png`.

34. **`media/`** — el video de demostración (`demo.mp4`) y su portada
    (`portada.jpg`), que el README muestra al inicio.

## 2. Las 10 ecuaciones más importantes

### 1. La trayectoria de la luz (geodésica nula de Schwarzschild)

$$
\frac{d^2u}{d\phi^2}+u=3Mu^2,\qquad u=\frac1r
$$

En forma vectorial, que es la que integra el programa:

$$
\frac{d\mathbf r}{d\lambda}=\mathbf v,\qquad
\frac{d\mathbf v}{d\lambda}=-\frac32\,r_s\,h^2\,\frac{\mathbf r}{r^5},\qquad
h=\lVert\mathbf r\times\mathbf v\rVert
$$

Sin el término $3Mu^2$, la luz viajaría en línea recta; ese término es toda la
relatividad general del problema. Es lo que dobla la luz y crea la sombra, el
anillo de fotones, el arco del disco por encima y por debajo del agujero, y las
estrellas estiradas.

**Dónde se usa:**

- [relativity.rs](src/scene/relativity.rs): `geodesic_acceleration` es la
  fórmula; `angular_momentum` y `photon_energy` calculan $h$ y la energía que se
  conservan.
- [blackhole.rs](src/scene/blackhole.rs): `Photon::advance` la evalúa cuatro
  veces por paso y `step_length` la usa para elegir el tamaño del paso.
- [raymarch.rs](src/render/raymarch.rs): `march` avanza con ella cada rayo de la
  cámara, cada rebote en la nave y los rayos que capturan la luz del entorno
  (`capture_environment`).
- [verify.rs](src/verify.rs): la compara con resultados exactos de la
  relatividad.

### 2. El tamaño del agujero: radio de Schwarzschild y sombra

$$
r_s=\frac{2GM}{c^2},\qquad
r_{\text{fotones}}=1.5\,r_s,\qquad
r_{\text{ISCO}}=3\,r_s,\qquad
b_c=\frac{3\sqrt3}{2}\,r_s\approx2.6\,r_s
$$

$r_s$ es el horizonte de eventos: de ahí no sale ni la luz. El proyecto lo usa
como unidad ($r_s=1$). A 1.5 $r_s$ la luz puede quedar orbitando (esfera de
fotones) y a 3 $r_s$ está la órbita estable más cercana, donde empieza el disco.
La sombra se ve con radio $b_c\approx2.6\,r_s$, más grande que el horizonte,
porque la gravedad también atrapa la luz que pasa cerca.

**Dónde se usa:**

- [config.rs](src/config.rs): `SCHWARZSCHILD_RADIUS`, `BLACK_HOLE_MASS`,
  `PHOTON_SPHERE_RADIUS`, `SHADOW_RADIUS` y `DISK_INNER_RADIUS` guardan estos
  valores.
- [relativity.rs](src/scene/relativity.rs): todas sus fórmulas usan $r_s$ y $M$.
- [blackhole.rs](src/scene/blackhole.rs): `captured` decide que un rayo cayó si
  cruza el horizonte (con `sd_sphere` de [sdf.rs](src/math/sdf.rs)) o si entra
  a la esfera de fotones yendo hacia adentro.
- [disk.rs](src/scene/disk.rs) y [raymarch.rs](src/render/raymarch.rs): el gas
  empieza en 3 $r_s$.
- [verify.rs](src/verify.rs): mide $b_c$ y la órbita de la esfera de fotones.

### 3. Runge–Kutta de cuarto orden

$$
k_1=F(Y_n),\quad
k_2=F\!\left(Y_n+\tfrac{\Delta\lambda}{2}k_1\right),\quad
k_3=F\!\left(Y_n+\tfrac{\Delta\lambda}{2}k_2\right),\quad
k_4=F(Y_n+\Delta\lambda\,k_3)
$$

$$
Y_{n+1}=Y_n+\frac{\Delta\lambda}{6}\left(k_1+2k_2+2k_3+k_4\right)
$$

Runge–Kutta no es una ley de la física sino un método numérico: aproxima la
solución de una ecuación diferencial avanzando en pasos pequeños. $Y$ es la
posición y velocidad del fotón, y $F$ su derivada (la ecuación 1).
La ecuación de la luz no se puede resolver a mano para cada píxel: RK4 la avanza
paso a paso, con un error mucho menor que el método de Euler. El paso se acorta
donde la luz se curva más.

**Dónde se usa:**

- [blackhole.rs](src/scene/blackhole.rs): `Photon::advance` es el paso de RK4
  y `step_length` elige su tamaño.
- [raymarch.rs](src/render/raymarch.rs): `march` llama a `advance` en cada paso
  de cada rayo, y `volume_step` acorta el paso cerca del gas.
- [verify.rs](src/verify.rs): integra los rayos de prueba con el mismo método.

### 4. Emisión y absorción del gas (transferencia radiativa)

$$
\alpha_i=1-e^{-\tau_i},\qquad
C_{i+1}=C_i+T_i\,\alpha_i\,S_i,\qquad
T_{i+1}=T_i\,(1-\alpha_i),\qquad
C_{\text{final}}=C_N+T_N\,C_{\text{fondo}}
$$

Cada paso del rayo dentro del gas aporta luz propia $S$ y tapa lo que hay detrás
según su opacidad $\alpha$, que depende de la profundidad óptica $\tau$ (cuánto
gas atraviesa). La transmitancia $T$ dice cuánta luz de atrás sigue llegando.
Por esto el disco se ve como un volumen de gas brillante y semitransparente, y
no como una superficie plana.

**Dónde se usa:**

- [disk.rs](src/scene/disk.rs): `prepare` calcula la emisión $S$ y la
  profundidad óptica $\tau$ de cada muestra, y `DiskSample::shade` calcula
  $\alpha$ con la turbulencia del instante.
- [raymarch.rs](src/render/raymarch.rs): `composite` acumula $C$ y $T$ desde la
  cámara hacia el fondo; `shade_span` y `shade_surface` suman el cielo o la
  nave con la transmitancia que queda; `capture_environment` la usa para la luz
  de la nave.

### 5. El corrimiento de la luz (Doppler y gravedad)

$$
g=\frac{\nu_{\text{obs}}}{\nu_{\text{em}}}=
\frac{\sqrt{1-3M/R}}{\left(1-\Omega\,b_y\right)\sqrt{1-r_s/r_{\text{obs}}}},
\qquad \Omega=\sqrt{\frac{M}{R^3}}
$$

$\Omega$ es la velocidad angular del gas en órbita y $b_y$ indica hacia dónde va
el fotón respecto del giro. El factor $(1-\Omega b_y)$ es el efecto Doppler: el
lado del disco que se acerca tiene $g>1$ y el que se aleja, $g<1$. La raíz de
arriba es la energía que pierde la luz al salir del pozo gravitatorio.

**Dónde se usa:**

- [relativity.rs](src/scene/relativity.rs): `redshift_factor` es la fórmula y
  `keplerian_omega` da $\Omega$.
- [blackhole.rs](src/scene/blackhole.rs): `from_camera` calcula $b_y$
  (`lz_over_e`) una sola vez por rayo, porque se conserva.
- [raymarch.rs](src/render/raymarch.rs): `march` lo pasa a cada muestra en un
  `RayFrame`.
- [disk.rs](src/scene/disk.rs): `prepare` calcula $g$ en cada muestra de gas, y
  `keplerian_omega` también hace girar la turbulencia en `GasTexture`.

### 6. Color y brillo observados (beaming relativista)

$$
T_{\text{obs}}=g\,T_{\text{em}},\qquad I_{\text{obs}}=g^4\,I_{\text{em}}
$$

Con la $g$ de la ecuación 5, el lado que se acerca se ve más caliente (más
blanco) y mucho más brillante: con $g=1.3$, el brillo se multiplica por
$1.3^4\approx2.9$. Por eso un lado del disco brilla mucho más que el otro.

**Dónde se usa:**

- [disk.rs](src/scene/disk.rs): `prepare` aplica la temperatura observada
  $gT$ y el brillo $(gT/7000\,\mathrm K)^4$ a cada muestra de gas.
- [blackbody.rs](src/math/blackbody.rs): `planckian_rgb` convierte la
  temperatura observada en color.
- [stars.rs](src/scene/stars.rs): lo mismo para el cielo, con
  `blueshift_from_infinity` de [relativity.rs](src/scene/relativity.rs): cerca
  del agujero las estrellas se ven más azules y brillantes. `intensity_shift`
  aplica ese $g^4$ también al skybox en [raymarch.rs](src/render/raymarch.rs).

### 7. La temperatura del disco

$$
T(R)\propto x^{-3/4}\left(1-x^{-1/2}\right)^{1/4},\qquad
x=\frac{R}{R_{\text{in}}},\quad R_{\text{in}}=3\,r_s
$$

Es el modelo de disco delgado con torque nulo en el borde interno: el gas de
adentro está más caliente, pero la temperatura cae a cero justo en el borde
(3 $r_s$), y el máximo queda un poco afuera, en $R=\tfrac{49}{36}\cdot3\approx4.1\,r_s$.
Se normaliza para que el pico sea 7000 K.

**Dónde se usa:**

- [relativity.rs](src/scene/relativity.rs): `disk_temperature` es la fórmula.
- [disk.rs](src/scene/disk.rs): `prepare` la evalúa en cada muestra de gas.
- [config.rs](src/config.rs): `THIN_DISK_PROFILE_PEAK` normaliza el máximo, y
  cada versión fija su temperatura de color de pico: 7000, 6500 y 5200 K.

### 8. Ley de Snell (la refracción de la cúpula)

$$
n_1\sin\theta_1=n_2\sin\theta_2,\qquad
\mathbf t=\eta\,\mathbf d+\left(\eta\cos\theta_1-\sqrt{1-\eta^2\left(1-\cos^2\theta_1\right)}\right)\mathbf n,
\qquad \eta=\frac{n_1}{n_2}
$$

La luz se dobla al entrar al vidrio ($n=1.5$) y otra vez al salir. Si lo de
dentro de la raíz es negativo, hay reflexión total interna y la luz rebota
adentro. Además el vidrio absorbe un poco: $A=e^{-\sigma\ell}$. Por eso a
través de la cúpula se ven las luces del tablero desplazadas.

**Dónde se usa:**

- [material.rs](src/scene/material.rs): `refract` es la ley de Snell en forma
  vectorial; devuelve `None` cuando hay reflexión total interna.
- [raymarch.rs](src/render/raymarch.rs): `shade_hit` refracta el rayo que entra
  a la cúpula.
- [endurance.rs](src/scene/endurance.rs): `through_glass` sigue el rayo por
  dentro del vidrio, lo refracta al salir, resuelve la reflexión total interna y
  aplica Beer-Lambert con `GLASS_ABSORPTION` de [config.rs](src/config.rs).

### 9. Reflexión y Fresnel

$$
\mathbf r=\mathbf d-2(\mathbf d\cdot\mathbf n)\,\mathbf n,\qquad
F=F_0+(1-F_0)\left(1-\cos\theta\right)^5
$$

$\mathbf r$ es la dirección del rayo reflejado (el ángulo de entrada es igual al
de salida). $F$, la aproximación de Schlick, dice qué fracción de la luz se
refleja: $F_0$ es la reflectividad del material y, en ángulo rasante, todo
refleja más, por eso los bordes brillan. En este proyecto el rayo reflejado
vuelve a ser luz curvada: el casco refleja el disco deformado por la lente.

**Dónde se usa:**

- [material.rs](src/scene/material.rs): `reflect` y `fresnel` son las
  fórmulas.
- [endurance.rs](src/scene/endurance.rs): `shade` reparte la luz entre
  reflexión, refracción y difuso con Fresnel, y calcula la dirección del reflejo
  borroso; `through_glass` refleja el rayo dentro del vidrio cuando hay
  reflexión total interna.
- [raymarch.rs](src/render/raymarch.rs): `shade_hit` lanza el rayo reflejado
  como una geodésica nueva.

### 10. Sphere tracing con funciones de distancia (SDF)

$$
t_{k+1}=t_k+d(\mathbf p_k),\qquad
\mathbf p_k=\mathbf a+t_k\,\hat{\mathbf u},\qquad
\text{impacto si } d(\mathbf p_k)<\varepsilon
$$

$d(\mathbf p)$ es la distancia del punto a la superficie más cercana de la nave
(negativa adentro). Como en una esfera de radio $d$ no hay nada, el rayo puede
avanzar esa distancia sin chocar. Así se encuentra la nave sin triángulos: toda
su geometría son fórmulas de cajas, cilindros, conos y esferas, y la normal es
el gradiente de $d$.

**Dónde se usa:**

- [endurance.rs](src/scene/endurance.rs): `Ship::intersect` hace la marcha,
  `evaluate` da la distancia a todas las piezas de la nave y `normal` saca su
  gradiente. `soft_shadow` y `ambient_occlusion` también marchan con esa
  distancia.
- [raymarch.rs](src/render/raymarch.rs): `march` llama a `ship.intersect` en
  cada tramo del rayo.
- [sdf.rs](src/math/sdf.rs): `sd_sphere`, la distancia a una esfera, que
  [blackhole.rs](src/scene/blackhole.rs) usa para detectar el horizonte.

**Otras que también aparecen:** armónicos esféricos (luz difusa de la nave),
microfacetas GGX (brillos), curva ACES (aspecto de película), lugar
planckiano (color del cuerpo negro), conservación de la energía y el momento
angular del fotón, fBm (turbulencia) y filtro gaussiano separable (bloom).

## 3. ¿Cómo se forma un agujero negro?

Una estrella vive en equilibrio: la gravedad la aprieta hacia adentro y la
energía de la fusión nuclear empuja hacia afuera. Cuando una estrella muy
masiva, de más de unas 20 veces la masa del Sol, se queda sin combustible, ese
empuje desaparece. Su núcleo colapsa en menos de un segundo mientras las capas
exteriores salen disparadas en una supernova. Si el núcleo que queda tiene más
de unas 3 masas solares, ninguna fuerza conocida puede frenar el colapso: la
materia se comprime hasta quedar dentro de su radio de Schwarzschild y desde
ahí ni la luz puede escapar. Esa frontera es el horizonte de eventos.

Los agujeros supermasivos, como Gargantua (100 millones de soles en la
película), viven en el centro de las galaxias y crecieron tragando gas y
fusionándose con otros agujeros. El agujero en sí no brilla: lo que brilla es
el disco de acreción, gas que gira a su alrededor y se calienta a miles de
grados por fricción mientras cae. Eso es lo que simula este proyecto.

### Analogías

**Convertir la Tierra en agujero negro sería como comprimir el planeta entero
(océanos, montañas y 8 mil millones de personas) hasta el tamaño de una
canica.**

| Si comprimieras… | …tendría que caber en | Para comparar |
| --- | --- | --- |
| La Tierra | una esfera de 1.8 cm de diámetro | una canica |
| El Sol | una esfera de 6 km de diámetro | una ciudad pequeña |
| 10 elefantes (60 toneladas) | una esfera de 9 × 10⁻²³ m de radio | 10 millones de veces más chica que un protón |
| Una persona (70 kg) | una esfera de 10⁻²⁵ m de radio | unas 8 mil millones de veces más chica que un protón |
| TON 618 (66 mil millones de soles) | una esfera de 390 mil millones de km de diámetro (2600 UA) | si la Vía Láctea midiera lo que la Tierra, una bola de 5 metros |

Con 10 elefantes metidos en un ácaro de 0.3 mm saldría una densidad de unos
4 × 10¹⁵ kg/m³. Eso es millones de veces más denso que una enana blanca y se
acerca a la densidad de un núcleo atómico, pero todavía no es un agujero negro:
para serlo, el ácaro tendría que ser un trillón de veces más chico (un millón
de millones de millones (10¹⁸)).

Dos datos curiosos sobre Gargantua:

- Su horizonte mediría unos 2 UA (295 millones de km). Si estuviera en lugar
  del Sol, se tragaría las órbitas de la Tierra y de Marte.
- Su "densidad promedio" (masa entre el volumen del horizonte) sería de unos
  1800 kg/m³, la de un ladrillo. Cuanto más grande es un agujero negro, menos
  denso es en promedio. Es solo un número curioso: la materia en realidad está
  en el centro.

### TON 618: uno de los agujeros negros más masivos que se conocen

La masa de los agujeros negros se mide en **masas solares** ($M_\odot$): una
masa solar es la masa del Sol, unos 2 × 10³⁰ kg, es decir, 333 000 Tierras.

TON 618 es un cuásar: un agujero negro gigante con un disco de acreción tan
caliente que brilla como más de 100 billones de soles (100 millones de
millones de soles (10¹⁴)). Su luz salió hace unos 10 500 millones de años. Su masa se estima en
**unos 66 mil millones de masas solares**. Para medirla se observa qué tan rápido gira el gas cerca del
agujero (por el ensanchamiento Doppler de sus líneas espectrales) y a qué
distancia está ese gas: a la misma distancia, cuanto más rápido gira, más masa
lo sostiene. Es una estimación con incertidumbre, pero todos los métodos dan
decenas de miles de millones de soles.

**TON 618 es como comprimir 66 mil millones de soles, tantos como todas las
estrellas de la Vía Láctea, en una esfera de 390 mil millones de km de diámetro
(2600 UA): una burbuja que la luz cruza en 15 días, cuando a la Vía Láctea le
toma 100 000 años. Si la Vía Láctea fuera del tamaño de la Tierra, TON 618
sería una bola de 5 metros con la masa de todas sus estrellas.**

A lo ancho de esa esfera cabrían 43 sistemas solares en fila, o 280 mil soles
puestos uno junto a otro. Si TON 618 estuviera en lugar del Sol, la sonda
Voyager 1, el objeto humano más lejano (a unas 165 UA), quedaría 8 veces más
adentro de su horizonte.

Lo sorprendente es que esos 66 mil millones de soles, apretados sin dejar
espacio entre ellos, formarían una bola de unas 38 UA, del tamaño de la órbita
de Urano, y el horizonte de TON 618 es 70 veces más ancho que esa bola. Para
formar un agujero así no hace falta apretar la materia, como a la Tierra en una
canica: hace falta juntar muchísima. El horizonte crece en proporción a la
masa, pero su volumen crece con el cubo del tamaño. Por eso la "densidad
promedio" de TON 618 es tan baja como la del aire a 40 km de altura, donde
vuelan los globos estratosféricos.

| Comparación | TON 618 |
| --- | --- |
| Masa | 66 mil millones de soles (6.6 × 10¹⁰ $M_\odot$ ≈ 1.3 × 10⁴¹ kg) |
| Frente a Gargantua (100 millones de soles) | 660 veces más masivo |
| Frente a Sagitario A*, el agujero del centro de la Vía Láctea (4.3 millones de soles) | unas 15 000 veces más masivo |
| En Tierras | 22 mil billones de Tierras (22 mil millones de millones de Tierras (2.2 × 10¹⁶)) |
| En ballenas azules de 150 toneladas | 875 mil quintillones de ballenas (un 875 seguido de 33 ceros (8.75 × 10³⁵)) |
| En elefantes de 6 toneladas | 22 sextillones de elefantes (un 22 seguido de 36 ceros (2.2 × 10³⁷)) |
| Tamaño del horizonte | radio de unas 1300 UA (195 mil millones de km): 43 veces la órbita de Neptuno |
| Luz cruzando el horizonte | tardaría unos 15 días en recorrer su diámetro |
| "Densidad promedio" | unos 0.004 kg/m³: como el aire a 40 km de altura, 280 veces menos denso que al nivel del mar |

Para imaginar la cantidad de elefantes: si contaras un elefante por segundo
desde el Big Bang hasta hoy (13 800 millones de años), no llegarías ni cerca.
Tendrías que repetir la edad entera del universo unos 50 trillones de veces
(un 50 seguido de 18 ceros (5 × 10¹⁹)).

### Relación con el proyecto

En el código, $r_s=1$ es la unidad de longitud. Si el horizonte de Gargantua
mide 2 UA, el anillo de nuestra Endurance (0.02 $r_s$) tendría unos 6 millones
de km de radio. Es una licencia cinematográfica, igual que en la película, para
que la nave y el agujero quepan en la misma toma. Además, el agujero del
proyecto es de Schwarzschild (no gira), mientras que el de la película es de
Kerr (gira y arrastra el espacio a su alrededor).

## 4. Glosario

- **Agujero negro:** región del espacio donde la gravedad es tan fuerte que
  nada, ni la luz, puede salir. Su frontera es el horizonte de eventos.
- **Albedo:** el color propio de una superficie, la fracción de luz que refleja
  de forma difusa (mate). Un albedo de 0 es negro y uno de 1, blanco.
- **Anillo de fotones:** el aro fino y brillante pegado a la sombra. Es luz que
  dio una o más vueltas alrededor del agujero antes de llegar a la cámara.
- **Año luz:** la distancia que recorre la luz en un año: unos 9.5 billones de
  km (9.5 millones de millones de km (9.5 × 10¹²)).
- **Armónicos esféricos:** funciones que describen cómo cambia algo según la
  dirección, como las notas de una cuerda pero sobre una esfera. Con 9 números
  por color resumen toda la luz que llega a la nave.
- **Beaming relativista (efecto faro):** el gas que viene hacia nosotros a una
  fracción importante de la velocidad de la luz se ve mucho más brillante, y el
  que se aleja, más tenue. Por eso un lado del disco brilla más.
- **Bloom:** el halo alrededor de lo muy brillante, como al mirar una lámpara.
  Imita la luz que se dispersa dentro del lente de una cámara.
- **Caché:** datos guardados para no volver a calcularlos. Aquí guarda las
  trayectorias de los rayos mientras la cámara está quieta.
- **Corrimiento al rojo y al azul:** cambio del color de la luz. Si la fuente se
  aleja o la luz sale de un pozo gravitatorio, se corre al rojo (pierde
  energía); si la fuente se acerca, al azul.
- **Cuásar:** el centro de una galaxia con un agujero negro supermasivo cuyo
  disco de acreción brilla más que todas las estrellas de la galaxia juntas.
  TON 618 es uno.
- **Cubemap:** imagen hecha de las seis caras de un cubo que rodea la escena. Se
  usa para guardar el cielo del skybox.
- **Cuerpo negro:** objeto ideal que emite luz solo por su temperatura. Su color
  depende únicamente de qué tan caliente está: rojo, naranja, blanco o azulado.
- **Disco de acreción:** gas que gira alrededor del agujero negro mientras cae
  en espiral. La fricción lo calienta a miles de grados y lo hace brillar.
- **Ecuación diferencial:** ecuación que dice cómo cambia algo, en vez de cuánto
  vale. Por ejemplo, "la velocidad del fotón cambia según la gravedad".
  Resolverla es encontrar el recorrido completo, como la trayectoria de un rayo.
- **Efecto Doppler:** cambio de frecuencia (y de color, en la luz) por el
  movimiento de la fuente, como la sirena de una ambulancia que suena más aguda
  al acercarse y más grave al alejarse.
- **Esfera de fotones:** la distancia de 1.5 rs a la que la luz puede quedar
  dando vueltas alrededor del agujero, en una órbita inestable.
- **fBm (ruido fractal):** suma de varias capas de ruido a distintas escalas,
  cada una más fina y más tenue. Da el aspecto de nubes y turbulencia al gas.
- **Fresnel (efecto):** las superficies reflejan más cuando se miran de canto
  que de frente, como un lago que a lo lejos parece un espejo.
- **Geodésica:** el camino "más recto posible" en un espacio curvo. La luz
  siempre sigue geodésicas; cerca de un agujero negro el espacio-tiempo está
  tan curvado que esos caminos se doblan.
- **Geodésica nula:** la geodésica que sigue la luz. Se llama nula porque, para
  la luz, la "distancia" en el espacio-tiempo entre dos puntos de su camino es
  cero.
- **GGX (microfacetas):** modelo que imagina una superficie como millones de
  espejitos diminutos inclinados al azar. La rugosidad decide si el brillo es
  pequeño y nítido o grande y suave.
- **HDR (alto rango dinámico):** colores sin tope de brillo, como la luz real.
  El disco puede ser miles de veces más brillante que las estrellas.
- **Horizonte de eventos:** la frontera del agujero negro, a 1 rs del centro en
  este proyecto. Lo que la cruza ya no puede salir.
- **Índice de refracción:** cuánto se frena la luz dentro de un material; decide
  cuánto se dobla al entrar. Aire, cerca de 1; agua, 1.33; vidrio, 1.5.
- **ISCO (órbita circular estable más interna):** la órbita más cercana en la
  que el gas puede girar sin caer, a 3 rs. Ahí empieza el disco.
- **Kerr (agujero de):** agujero negro que gira. El Gargantua de la película es
  de Kerr; el de este proyecto es de Schwarzschild.
- **Lente gravitacional:** la deformación de la imagen de lo que está detrás de
  un objeto muy masivo, porque su gravedad curva la luz.
- **Masa solar ($M_\odot$):** la masa del Sol, unos 2 × 10³⁰ kg. Es la unidad
  con que se mide la masa de las estrellas y de los agujeros negros.
- **Nivel de detalle (LOD):** reducir el detalle de una textura cuando el objeto
  está lejos, para que no parpadee.
- **Oclusión ambiental:** el oscurecimiento de rincones y rendijas, adonde llega
  menos luz del entorno.
- **Parámetro afín ($\lambda$):** la variable con que se avanza un rayo de luz
  a lo largo de su geodésica; hace el papel del tiempo en la integración.
- **Parámetro de impacto ($b$):** la distancia a la que un rayo pasaría del
  centro del agujero si no se curvara. Si es menor que 2.6 rs, el rayo cae.
- **Profundidad óptica ($\tau$):** cuánto material opaco atraviesa la luz. Con
  $\tau=1$ pasa el 37 % de la luz; con $\tau=5$, menos del 1 %.
- **Radio de Schwarzschild ($r_s$):** el radio del horizonte de un agujero negro
  sin giro, $r_s=2GM/c^2$. En el proyecto es la unidad de longitud.
- **Raymarching:** recorrer un rayo en muchos pasos pequeños en vez de calcular
  su choque de una sola vez. Aquí es necesario porque los rayos se curvan.
- **Reflexión total interna:** cuando la luz intenta salir de un material más
  denso (como el vidrio) con un ángulo muy rasante, no sale: se refleja entera
  hacia adentro.
- **Refracción:** el cambio de dirección de la luz al pasar de un material a
  otro, como un lápiz que parece quebrado dentro de un vaso de agua.
- **RGBE:** formato que guarda un color HDR en 4 bytes: tres valores de color y
  un exponente común.
- **Runge–Kutta (RK4):** método numérico para resolver ecuaciones diferenciales
  paso a paso. No da una fórmula de la solución: parte de un punto conocido y,
  en cada paso, mide la pendiente cuatro veces (al inicio, dos veces a la mitad
  y al final) y las promedia. Es mucho más preciso que el método de Euler, que
  la mide una sola vez. En el proyecto avanza cada rayo de luz por su
  trayectoria curva.
- **Schwarzschild (métrica de):** la solución de las ecuaciones de Einstein para
  una masa esférica que no gira. Describe cómo se curva el espacio-tiempo
  alrededor del agujero de este proyecto.
- **SDF (función de distancia con signo):** fórmula que da la distancia de un
  punto a una superficie: positiva afuera y negativa adentro. Con ellas se
  modela la nave sin triángulos.
- **Skybox:** el fondo lejano de la escena, el cielo, guardado como un cubo que
  rodea todo.
- **Sombra del agujero negro:** la zona oscura que se ve. Mide 2.6 veces el
  horizonte, porque el agujero también se traga la luz que pasa cerca.
- **Specular (brillo especular):** el brillo concentrado que aparece donde una
  superficie refleja directamente una luz, como el destello en una manzana.
- **Sphere tracing:** forma de recorrer un rayo con una SDF: en cada paso avanza
  la distancia a la superficie más cercana, un salto seguro porque no hay nada
  más cerca.
- **Supernova:** la explosión de una estrella masiva al final de su vida. Su
  núcleo puede convertirse en un agujero negro.
- **Textura procedural:** textura generada con fórmulas en vez de una imagen,
  como los paneles, las celdas solares o las losetas de la nave.
- **Tonemapping:** convertir colores HDR, sin tope, en colores que una pantalla
  puede mostrar, sin quemar lo brillante.
- **Transmitancia ($T$):** la fracción de luz que logra atravesar un medio.
  Empieza en 1 y baja a medida que el rayo cruza gas.
- **Trazado de rayos (raytracing):** generar una imagen siguiendo rayos de luz
  desde la cámara hacia la escena, uno por píxel.
- **Unidad astronómica (UA):** la distancia media entre la Tierra y el Sol,
  unos 150 millones de km.
