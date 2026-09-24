# Gargantua — Agujero negro de Schwarzschild en CPU

Gráficas por Computadora · Lázaro Díaz, 24713

Render interactivo en Rust de un agujero negro y su disco de acreción, inspirado
en la apariencia de Gargantua. La lente gravitacional se obtiene integrando
trayectorias de luz; el plasma es un volumen procedural con rotación diferencial,
emisión y absorción. El color incorpora Doppler y corrimiento gravitacional.

Todo se calcula en CPU con `rayon`, `glam` y `minifb`. El disco y las estrellas
se generan durante la ejecución, sin imágenes ni GIFs usados como fondo.

## Dos versiones con `V`

Ambas versiones forman parte del proyecto y se conservan para seguir ampliándolo.
**`V` alterna entre ellas durante la ejecución**, manteniendo la cámara y el
tiempo de animación. El título de la ventana indica cuál está activa.

| Característica | Original | Variante — predeterminada |
| --- | --- | --- |
| Disco | Plasma animado y contraste Doppler | Mismo modelo, con temperatura de color algo más cálida |
| Temperatura de color de pico, antes del corrimiento | 7000 K | 6500 K |
| Estrellas | Fondo discreto y fijo respecto al mundo | Fondo más visible, con deriva angular a través de la lente |
| Gas exterior | Atmósfera del disco térmico | Envoltura gris adicional que se desvanece entre 8 y 15 radios de Schwarzschild |
| Costo | Menor | Más muestras de gas y evaluación del cielo por frame |

Las dos versiones comparten las geodésicas, el sentido de giro del plasma,
el zoom suave y los controles. La versión original también deforma las estrellas
al cambiar la cámara; la deriva del fondo de la variante permite apreciar ese
efecto incluso con la cámara quieta.

## Ejecutar

Requiere Rust y Cargo, además de un entorno gráfico para abrir la ventana.

```sh
cargo run --release
```

Para iniciar con la versión original:

```sh
cargo run --release -- --original
```

| Acción | Control |
| --- | --- |
| Alternar original / variante | V |
| Orbitar | Arrastrar con el mouse o W/A/S/D |
| Acercar / alejar | Rueda o flechas arriba/abajo |
| Vista de canto, cinematográfica | 1 |
| Vista inclinada a 30° | 2 |
| Vista casi polar, 87° | 3 |
| Pausar / continuar gas y estrellas | Espacio |
| Restablecer cámara | R |
| Salir | Esc |

## Cómo se forma la imagen

```text
Cámara → geodésicas → emisión y absorción del gas → cielo de fondo
       → historial temporal corto → bloom → exposición y tonemap → ventana
```

Las trayectorias y las muestras del volumen se guardan en una caché por vista.
Con la cámara quieta se preparan dos muestras subpíxel y se vuelve a calcular
su iluminación al tiempo actual. La textura del gas, de 512×256, se regenera
en cada frame. En la variante también se actualiza el cielo.

Mover la cámara o cambiar de versión invalida la caché y el historial temporal.
Durante el movimiento se reduce la resolución interna a una escala de 0.33;
en reposo se recupera la resolución completa. El bloom mantiene su tamaño
respecto a la ventana para evitar cambios bruscos en el halo.

## Ecuaciones utilizadas

Las expresiones siguientes corresponden al código del proyecto. Se distinguen
las ecuaciones de propagación de luz de las aproximaciones para representar
el gas y la respuesta de la cámara.

### 1. Unidades y radios característicos

Se usan unidades geométricas con $G=c=1$ y se fija:

$$
r_s=\frac{2GM}{c^2}=1,\qquad M=\frac12.
$$

El horizonte, la esfera de fotones, la órbita circular estable más interna
(ISCO) y el parámetro de impacto crítico son:

$$
r_H=r_s=1,\quad r_{\mathrm{ph}}=3M=1.5,\quad
r_{\mathrm{ISCO}}=6M=3,\quad b_c=3\sqrt3 M\approx2.598076.
$$

$b_c$ describe el tamaño aparente de la sombra para un observador lejano;
no es el radio del horizonte.

En las fórmulas, $r=\lVert\mathbf r\rVert$ es el radio esférico,
$R=\sqrt{x^2+z^2}$ es el radio del disco y $y$ su altura. El eje del disco
es Y. $\lambda$ es el parámetro afín del rayo, no el tiempo de animación.

Implementación: [configuración](src/config.rs) y [relatividad](src/scene/relativity.rs).

### 2. Rayos de cámara, geodésicas y conservación

Para una dirección local unitaria $\mathbf n$, sus componentes radial y
tangencial se convierten a la parametrización del integrador mediante:

$$
\mathbf n_\parallel=(\mathbf n\cdot\hat{\mathbf r})\hat{\mathbf r},\qquad
\mathbf n_\perp=\mathbf n-\mathbf n_\parallel,
$$

$$
\mathbf v_0=\mathbf n_\perp+
\sqrt{1-\frac{r_s}{r_{\mathrm{obs}}}}\,\mathbf n_\parallel.
$$

La energía local inicial del fotón se normaliza a uno. Durante el trazado
se conservan el momento angular y la energía:

$$
\mathbf h=\mathbf r\times\mathbf v,\qquad
E^2=\lVert\mathbf v\rVert^2-\frac{r_s\lVert\mathbf h\rVert^2}{r^3}.
$$

La geodésica nula de Schwarzschild, en su plano orbital y con $u=1/r$, cumple:

$$
\frac{d^2u}{d\phi^2}+u=3Mu^2.
$$

La forma vectorial que integra el programa es:

$$
\frac{d\mathbf r}{d\lambda}=\mathbf v,\qquad
\frac{d\mathbf v}{d\lambda}=
-\frac32\,r_s\lVert\mathbf h\rVert^2\frac{\mathbf r}{r^5}.
$$

El módulo de $\mathbf v$ no es una velocidad física local y no se fuerza a uno
después de cada paso. La curvatura de los rayos genera los arcos del disco
y la deformación de las estrellas.

Implementación: [fotones](src/scene/blackhole.rs) y [relatividad](src/scene/relativity.rs).

### 3. Integración Runge–Kutta de cuarto orden

Para $\mathbf Y=(\mathbf r,\mathbf v)$ y
$F(\mathbf Y)=(\mathbf v,\mathbf a(\mathbf r))$:

$$
\begin{aligned}
k_1&=F(\mathbf Y_n),\\
k_2&=F(\mathbf Y_n+\tfrac12\Delta\lambda k_1),\\
k_3&=F(\mathbf Y_n+\tfrac12\Delta\lambda k_2),\\
k_4&=F(\mathbf Y_n+\Delta\lambda k_3),\\
\mathbf Y_{n+1}&=\mathbf Y_n+
\frac{\Delta\lambda}{6}(k_1+2k_2+2k_3+k_4).
\end{aligned}
$$

El paso base se limita por curvatura y radio:

$$
\Delta\lambda_{\mathrm{base}}=
\operatorname{clamp}\left(
\min\left(0.06\frac{\lVert\mathbf v\rVert}{\lVert\mathbf a\rVert},\;0.10r\right),
\;0.015,\;5\right).
$$

Para aceleración casi nula se usa 5 como límite de curvatura. El muestreo del
volumen reduce además el paso según su espesor, para no atravesar el núcleo
delgado sin tomar muestras. Se permiten hasta 512 pasos por rayo; la dirección
de salida se estima cuando el rayo supera $60r_s$ y se aleja del centro.

Implementación: [integrador](src/scene/blackhole.rs) y [raymarch](src/render/raymarch.rs).

### 4. Rotación orbital, Doppler y corrimiento gravitacional

La velocidad angular de una órbita circular ecuatorial es:

$$
\Omega(R)=\sqrt{\frac{M}{R^3}}.
$$

Para el gas y un observador estático, el factor de frecuencia utilizado es:

$$
g=\frac{\nu_{\mathrm{obs}}}{\nu_{\mathrm{em}}}=
\frac{\sqrt{1-3M/R}}
{(1-\Omega(R)b_y)\sqrt{1-r_s/r_{\mathrm{obs}}}},\qquad
b_y=\frac{L_y}{E}=-\frac{(\mathbf r\times\mathbf v)_y}{E}.
$$

El signo negativo aparece porque se traza desde la cámara hacia la fuente,
en sentido contrario al fotón recibido. En el código, el campo que guarda
$b_y$ conserva el nombre `lz_over_e`, aunque el eje empleado es Y.

$$
T_{\mathrm{obs}}=gT_{\mathrm{em}},\qquad
\frac{I_\nu}{\nu^3}=\mathrm{constante},\qquad
I_{\mathrm{bol,obs}}=g^4 I_{\mathrm{bol,em}}.
$$

El factor $g^4$ se aplica una sola vez. Produce un lado más brillante y azulado
al acercarse, y uno más tenue y cálido al alejarse. Visto exactamente desde el
eje, el Doppler longitudinal se anula. La fórmula ecuatorial se extrapola al
pequeño espesor del gas.

Implementación: [relatividad](src/scene/relativity.rs) y [disco](src/scene/disk.rs).

### 5. Temperatura y color del plasma

El disco usa un perfil térmico newtoniano con torque nulo en su borde interno:

$$
x=\frac{R}{R_{\mathrm{in}}},\qquad R_{\mathrm{in}}=3,\qquad
T_{\mathrm{em}}(R)=\frac{7000\,\mathrm K}{0.487872}
x^{-3/4}(1-x^{-1/2})^{1/4}.
$$

Se define $T_{\mathrm{em}}=0$ para $R\le R_{\mathrm{in}}$.
El máximo está en $R=(49/36)R_{\mathrm{in}}$.

La cromaticidad se obtiene con una aproximación polinómica del lugar
planckiano, $P(T)$, implementada en [blackbody.rs](src/math/blackbody.rs).
La temperatura se acota a 1667–25000 K. Sus coordenadas CIE $(x_c,y_c)$
se convierten a XYZ y a RGB lineal:

$$
X=\frac{x_c}{y_c},\qquad Y=1,\qquad Z=\frac{1-x_c-y_c}{y_c},
$$

$$
P(T)=\max\left(\begin{bmatrix}
3.2406&-1.5372&-0.4986\\
-0.9689&1.8758&0.0415\\
0.0557&-0.2040&1.0570
\end{bmatrix}\begin{bmatrix}X\\Y\\Z\end{bmatrix},\;0\right).
$$

La fuente térmica RGB utilizada en el volumen es:

$$
S_{\mathrm{th}}=P(sgT_{\mathrm{em}})
\left(\frac{gT_{\mathrm{em}}}{7000\,\mathrm K}\right)^4,\qquad
s=\begin{cases}1&\text{original},\\6500/7000&\text{variante}.\end{cases}
$$

El ajuste de la variante cambia la temperatura de color conservando la
normalización de brillo. Esta es una representación RGB aproximada, no una
integración espectral completa de la ley de Planck.

### 6. Espesor del disco y gas gris exterior

La transición suave empleada en los bordes es:

$$
\mathcal S(a,b;R)=q^2(3-2q),\qquad
q=\operatorname{clamp}\left(\frac{R-a}{b-a},0,1\right).
$$

Con alturas $H_c=0.004R$ y $H_a=0.035R$, el perfil de opacidad térmica es:

$$
W(R)=\mathcal S(3,3.24;R)\,[1-\mathcal S(7.5,13;R)],
$$

$$
D_{\mathrm{th}}=W(R)\left[
\frac{8}{H_c}e^{-y^2/(2H_c^2)}+
\frac{0.035}{H_a}e^{-y^2/(2H_a^2)}\right].
$$

La variante añade una envoltura con $H_o=0.035R$:

$$
D_o=\frac{0.07}{H_o}\,
\mathcal S(8,13;R)\,[1-\mathcal S(13,15;R)]\,e^{-y^2/(2H_o^2)},
$$

$$
S_o=0.11\,g^4(0.94,0.96,1.0)\,e^{-0.32\max(R-8,0)}.
$$

Esta fuente gris aproxima luz dispersada que se atenúa hacia afuera. Los
perfiles se truncan a 3.5 alturas de su atmósfera; el medio térmico termina
en $R=13$ y la envoltura en $R=15$. En la versión original, $D_o=0$.

La opacidad y la fuente combinadas son:

$$
D=D_{\mathrm{th}}+D_o,\qquad
S=\frac{D_{\mathrm{th}}S_{\mathrm{th}}+D_oS_o}{D}.
$$

Se omiten muestras de densidad despreciable para evitar divisiones por cero.
Implementación: [disco](src/scene/disk.rs).

### 7. Emisión, absorción y composición del volumen

La longitud comóvil y la profundidad óptica de cada segmento se aproximan por:

$$
\Delta\ell_{\mathrm{em}}=\frac{\Delta\lambda}{g},\qquad
\tau=\frac{D}{\sqrt{2\pi}}\frac{\Delta\lambda}{g}.
$$

El campo procedural del gas $m$ modula la opacidad y la emisión:

$$
\alpha=1-e^{-\tau(0.4+0.6m)},\qquad S_{\mathrm{gas}}=mS.
$$

Se compone desde la cámara hacia el fondo, empezando con color $C_0=0$
y transmitancia $\mathcal T_0=1$:

$$
C_{i+1}=C_i+\mathcal T_i\alpha_i S_{\mathrm{gas},i},\qquad
\mathcal T_{i+1}=\mathcal T_i(1-\alpha_i),
$$

$$
C_{\mathrm{final}}=C_N+\mathcal T_N C_{\mathrm{cielo}}.
$$

La composición se detiene si la transmitancia cae por debajo de 0.004.
Un rayo capturado no recibe contribución del cielo.

Implementación: [disco](src/scene/disk.rs) y [raymarch](src/render/raymarch.rs).

### 8. Ruido, filamentos y movimiento del gas

El ruido de valor $N(\mathbf q)\in[-1,1]$ interpola una tabla determinista
usando el suavizado $f(a)=a^2(3-2a)$. La suma de cinco octavas es:

$$
\operatorname{fBm}(\mathbf q)=\frac12+\frac12
\frac{\sum_{k=0}^{4}2^{-(k+1)}N(2^k\mathbf q)}
{\sum_{k=0}^{4}2^{-(k+1)}}.
$$

El ángulo del disco se introduce como seno y coseno para evitar una costura
al completar una vuelta. Una deformación de dominio $\mathbf w$, obtenida
con tres muestras de ruido, produce cada capa:

$$
L=0.8\operatorname{fBm}(\mathbf q+1.15\mathbf w)
+0.2\left[1-\left|N(2.8\mathbf q+2\mathbf w)\right|\right]-0.035.
$$

Se mezclan dos capas de edades desfasadas, con período $P=5$ segundos:

$$
p_a=\operatorname{fract}(t/P),\quad
p_b=\operatorname{fract}(t/P+0.5),\quad w_a=\sin^2(\pi p_a),
$$

$$
\phi_j=\phi+8\Omega(R)(p_j-0.5)P,\qquad
F=w_aL_a+(1-w_a)L_b,\qquad
m=1+0.96\left[e^{8(F-0.5)}-1\right].
$$

Los campos se renuevan cuando su peso es cero, evitando reinicios visibles.
La rotación depende del radio: las partes interiores se mueven más rápido.
El factor 8 acelera el tiempo de animación; el Doppler sigue usando
la velocidad orbital del modelo físico.

Implementación: [ruido](src/math/noise.rs) y [textura del disco](src/scene/disk.rs).

### 9. Estrellas y movimiento aparente del entorno

El cielo se consulta con la dirección de salida de la geodésica,
$\mathbf d_{\mathrm{esc}}$. En la variante se transforma con:

$$
\mathbf d_{\mathrm{cielo}}(t)=
R_z(0.23)R_y(0.012t)R_z(-0.23)\mathbf d_{\mathrm{esc}}.
$$

Los ángulos están en radianes. Esta deriva angular representa movimiento
relativo del fondo, mientras la métrica del agujero permanece estática.
El estiramiento de las estrellas procede del mapa de geodésicas.

Cada estrella tiene un perfil $B_\star=B_0\eta^3e^{-(d/s_\star)^2}$,
donde $d$ es la distancia al centro de su celda, $s_\star$ su tamaño y
$\eta\in[0,1)$ un valor determinista que distribuye los brillos.
El corrimiento para luz procedente de muy lejos es:

$$
g_\infty=\frac1{\sqrt{1-r_s/r_{\mathrm{obs}}}},\qquad
T_{\star,\mathrm{obs}}=g_\infty T_\star,\qquad
I_{\star,\mathrm{obs}}=g_\infty^4I_\star.
$$

Implementación: [estrellas](src/scene/stars.rs).

### 10. Historial temporal, bloom y tonemap

Para reducir ruido sin acumular segundos de plasma superpuesto:

$$
w_t=\min(e^{-\Delta t/0.045},0.72),\qquad
C_h=(1-w_t)C_{\mathrm{actual}}+w_tC_{\mathrm{anterior}}.
$$

El historial se guarda antes del bloom y se descarta al cambiar de vista.
Para el bloom se calcula la luminancia del color promedio $\bar C$ de cada
bloque y se extrae su parte brillante:

$$
Y=0.2126\bar C_R+0.7152\bar C_G+0.0722\bar C_B,\qquad
B_0=\bar C\frac{\max(Y-0.6/5,0)}{\max(Y,10^{-4})}.
$$

Se forman seis niveles reducidos y se desenfocan con un núcleo gaussiano
separable, normalizado, de radio 4 y $\sigma=2$:

$$
K(i)=\frac{e^{-i^2/(2\sigma^2)}}{\sum_{j=-4}^{4}e^{-j^2/(2\sigma^2)}},\qquad
C_b=C_h+0.38\sum_{k=0}^{5}\frac{0.72^k}{\sum_{j=0}^{5}0.72^j}B_k.
$$

Aquí $B_k$ representa cada nivel ya desenfocado y reescalado. Finalmente:

$$
C_e=\max(5C_b,0),\qquad
C_{\mathrm{pantalla}}=
\left(\frac{C_e}{1+\max(C_{e,R},C_{e,G},C_{e,B})}\right)^{1/2.2}.
$$

La división usa un factor común para RGB, preservando las proporciones
de color antes de aplicar gamma. El halo y la exposición representan la
presentación de la cámara; no modifican las trayectorias de luz.

Implementación: [historial](src/render/accumulate.rs), [bloom](src/render/bloom.rs)
y [tonemap](src/render/tonemap.rs).

### 11. Zoom suave

La distancia de cámara cambia de forma multiplicativa:

$$
d_{\mathrm{nuevo}}=\operatorname{clamp}
\left(d_{\mathrm{actual}}e^{-0.05\delta},\;2.2,\;120\right).
$$

$\delta>0$ acerca la cámara. La entrada de rueda se limita a 2.5 pasos por
frame; el zoom por teclado también limita el tiempo de frame usado en su
cálculo. Esto evita saltos grandes al comenzar a acercarse o alejarse.

Implementación: [cámara](src/camera.rs) y [entrada](src/input.rs).

## Validación y capturas

```sh
cargo test --release
cargo clippy --all-targets -- -D warnings
cargo run --release -- --verify
```

Las 15 pruebas cubren continuidad del ruido y del flujo, sentido de advección,
Doppler, integración de opacidad, conservación del color, caché, animación
del cielo, rayos capturados y desvanecimiento del gas gris.

La verificación analítica comprueba $b_c$, conservación de energía y momento
angular, la órbita de fotones en $r=1.5$ y la deflexión en campo débil:

$$
\alpha(b)\approx\frac{4M}{b}+\frac{15\pi}{4}\left(\frac{M}{b}\right)^2,
\qquad M/b\ll1.
$$

Esta última expresión es una referencia de prueba, no la fórmula usada para
curvar los rayos. En la revisión registrada, la deriva de los invariantes fue
del orden de $10^{-6}$ y la diferencia frente a esta serie fue de 0.425 %.
`--verify` devuelve un código de error si alguna comprobación falla.

Para medir arrastre real y cámara quieta, guardando el último frame:

```sh
cargo run --release -- --probe 30 frame.ppm 2 21.5 2
cargo run --release -- --probe 30 original.ppm 2 21.5 2 --original
```

Argumentos de `--probe`: frames, salida PPM, elevación en grados, distancia
en radios de Schwarzschild y tiempo inicial en segundos.

Para exportar una secuencia a 30 muestras por segundo o una captura individual:

```sh
cargo run --release -- --sequence artifacts/secuencia 90 2 21.5 0 960 540
cargo run --release -- --sequence artifacts/captura 1 30 36 2 1920 1080
```

Argumentos de `--sequence`: directorio, frames, elevación, distancia, tiempo
inicial, ancho y alto. Se puede añadir `--original` al final para usar la otra
versión. El directorio de salida se crea automáticamente.

Los PPM contienen el render sin compresión. Las exportaciones y respaldos
locales en `artifacts/` se excluyen de Git. Los FPS de reproducción de un
GIF o video exportado no son una medición del rendimiento interactivo.

## Rendimiento

Medición local del 23 de septiembre de 2026: 20 frames a 960×540, elevación
de 2°, distancia de 21.5 radios y tiempo inicial de 2 segundos.

| Medida | Original | Variante |
| --- | --- | --- |
| Cámara quieta, después de preparar la lente | 69.7 ms/frame · 14.4 FPS | 80.1 ms/frame · 12.5 FPS |
| Arrastre real, promedio completo a escala 0.33 | 73.2 ms/frame · 13.7 FPS | 90.6 ms/frame · 11.0 FPS |

Preparar las trayectorias cuesta más que sombrear una vista ya calculada y
se repite al mover la cámara o cambiar de versión. Los resultados dependen
del CPU, su temperatura, la resolución y la vista. La envoltura adicional
y el cielo animado aumentan el costo de la variante.

## Alcance del modelo

- El agujero es Schwarzschild: no gira ni produce arrastre de marcos. El
  plasma sí orbita. El Gargantua de la película se basa en un agujero de Kerr.
- El perfil térmico es una aproximación de disco delgado newtoniano, no el
  modelo relativista completo de Novikov–Thorne.
- La turbulencia, la densidad y los filamentos son procedurales. No se
  resuelven magnetohidrodinámica, acreción ni equilibrio vertical.
- La fuente gris aproxima luz dispersada. No calcula dispersión múltiple
  ni trata el gas frío como un emisor térmico visible.
- La escena se evalúa al tiempo de animación actual, sin retardos diferentes
  entre caminos de luz, polarización ni retroacción del gas sobre la métrica.
- La deriva de las estrellas no simula la traslación relativista del agujero.
  El modelo de color óptico tampoco reproduce las observaciones de radio del EHT.

## Estructura y futuras ampliaciones

| Archivo | Responsabilidad |
| --- | --- |
| [main.rs](src/main.rs) | Ventana, cambio con V, pausa, benchmark y exportación |
| [config.rs](src/config.rs) | Constantes físicas, apariencia de ambas versiones y cámara |
| [camera.rs](src/camera.rs), [input.rs](src/input.rs) | Proyección, órbita, zoom y controles |
| [relativity.rs](src/scene/relativity.rs), [blackhole.rs](src/scene/blackhole.rs) | Frecuencias, invariantes y geodésicas |
| [disk.rs](src/scene/disk.rs) | Emisión, absorción y textura del gas |
| [stars.rs](src/scene/stars.rs) | Cielo original y fondo animado de la variante |
| [noise.rs](src/math/noise.rs) | Ruido continuo y fBm |
| [raymarch.rs](src/render/raymarch.rs) | Muestreo del volumen y caché de trayectorias |
| [render/mod.rs](src/render/mod.rs) | Selección de versión y etapas del render |
| [accumulate.rs](src/render/accumulate.rs), [bloom.rs](src/render/bloom.rs), [tonemap.rs](src/render/tonemap.rs) | Historial, halo y presentación HDR |
| [framebuffer.rs](src/render/framebuffer.rs) | Buffers, reescalado y presentación |
| [verify.rs](src/verify.rs) | Comprobaciones analíticas |

Los parámetros `PREVIEW_*`, `OUTER_GAS_*` y `SKY_DRIFT_SPEED` controlan
la variante actual. La selección pasa por `Renderer::set_enhanced` y forma
parte de la clave de caché; esto permite comparar cambios visuales con el
modo original sin mezclar muestras o imágenes de ambas versiones.

## Referencias

- [NASA: visualización de un agujero negro y su disco](https://www.nasa.gov/universe/nasa-visualization-shows-a-black-holes-warped-world/), referencia visual de lente gravitacional y asimetría de brillo.
- [James, von Tunzelmann, Franklin y Thorne (2015)](https://arxiv.org/abs/1502.03808), descripción del render de *Interstellar* mediante haces de luz en Kerr. Este proyecto usa rayos individuales en Schwarzschild.
