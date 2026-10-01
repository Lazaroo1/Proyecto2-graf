# Gargantua — Agujero negro de Schwarzschild en CPU

Gráficas por Computadora · Lázaro Díaz, 24713

Render interactivo en Rust de un agujero negro y su disco de acreción, inspirado
en la apariencia de Gargantua. La lente gravitacional se obtiene integrando
trayectorias de luz; el plasma es un volumen procedural con rotación diferencial,
emisión y absorción. El color incorpora Doppler y corrimiento gravitacional.

La tercera versión, **Endurance**, recrea la toma de *Interstellar* en que la
nave, diminuta, pasa rozando el disco de Gargantua: seis materiales con
textura propia, reflexión, refracción, un skybox con la Vía Láctea, luz del
entorno capturada con geodésicas y una presentación de película. Los rayos que
rebotan en la nave siguen siendo geodésicas: el casco refleja el disco ya
deformado por la lente.

Todo el render se calcula en CPU con código propio y la biblioteca estándar
de Rust, sin librerías externas de cálculo, gráficos ni imágenes. La única
dependencia es `minifb`, autorizada por el profesor Pablo Koch, que se usa para
la ventana, la entrada y la presentación del buffer de píxeles. El disco, las
estrellas, el skybox y las texturas se generan durante la ejecución, sin
imágenes ni GIFs usados como fondo.

## Dependencias

La única dependencia directa es `minifb`, autorizada por el profesor Pablo
Koch. Solo gestiona la ventana, la entrada y la presentación del buffer de
píxeles; todo lo demás está escrito en el proyecto:

```toml
[dependencies]
minifb = "0.28"
```

- [vector.rs](src/math/vector.rs) implementa `Vec2`, `Vec3` y `Mat3`: productos
  punto y cruz, normalización, interpolación y rotaciones.
- [parallel.rs](src/parallel.rs) reparte bloques entre hilos persistentes con
  `std::thread`, canales y sincronización de `std::sync`. Cada bloque tiene
  un único escritor y todos los trabajos de una etapa terminan antes de
  pasar a la siguiente. Los bloques pequeños se procesan en el hilo llamador.
- Geodésicas, ruido, gas, cielo, skybox, la nave con sus materiales, bloom,
  tonemap y exportación PPM también se implementan en el proyecto, sin
  librerías externas de cálculo o render.

Para consultar las dependencias directas:

```sh
cargo tree --depth 1 --edges normal
```

`Cargo.lock` registra las dependencias transitivas que utiliza `minifb` para
su integración con las plataformas.

## Tres versiones con `V`

**`V` recorre las versiones original → variante → Endurance** durante la
ejecución. La original y la variante comparten cámara; al entrar o salir de la
Endurance la cámara cambia de objetivo, del agujero a la nave. El título de la
ventana indica la versión activa y sus teclas.

| Característica | Original | Variante — predeterminada | Endurance |
| --- | --- | --- | --- |
| Escena | Agujero y disco | Agujero y disco | La nave Endurance rozando el disco de Gargantua |
| Disco | Plasma animado y contraste Doppler | Filamentos volumétricos, canales oscuros y temperatura de color algo más cálida | El gas de la variante, en un mar de nubes más compacto y de color crema |
| Temperatura de color de pico, antes del corrimiento | 7000 K | 6500 K | 5200 K |
| Fondo | Estrellas discretas, fijas respecto al mundo | Skybox con la Vía Láctea y tres capas de estrellas, con deriva angular a través de la lente | El mismo skybox |
| Gas exterior | Atmósfera del disco térmico | Transición de gas cálido a gris desde 6 radios de Schwarzschild; se oscurece y desaparece suavemente hasta 19 | Como en la variante, con capas más delgadas |
| Textura del gas | Campo de 512×256 | Campo cilíndrico de 384×192×9, con variación en altura e interpolación trilineal | Igual que la variante |
| Superficies | — | — | Seis materiales con reflexión y refracción |
| Presentación | Bloom 0.38 y tonemap que conserva el color | Bloom 0.24 | Curva de película, destellos anamórficos, velo de lente, viñeta, grano y barras 2.39:1 |
| Costo | Menor | Más muestras de gas y evaluación del cielo por frame | Menor que la variante: las barras ahorran filas y los rayos atraviesan menos gas |

Las tres versiones comparten las geodésicas, el sentido de giro del plasma,
el zoom suave y los controles. La versión original también deforma las estrellas
al cambiar la cámara; la deriva del fondo de la variante permite apreciar ese
efecto incluso con la cámara quieta.

## Ejecutar

Requiere Rust y Cargo, además de un entorno gráfico para abrir la ventana.

```sh
cargo run --release
```

Para iniciar con la versión original o con la Endurance:

```sh
cargo run --release -- --original
cargo run --release -- --endurance
```

| Acción | Control |
| --- | --- |
| Cambiar de versión: original → variante → Endurance | V |
| Orbitar alrededor del agujero, o de la nave en la Endurance | Arrastrar con el mouse o W/A/S/D |
| Acercar / alejar | Rueda o flechas arriba/abajo |
| Vista 1 — agujero: de canto, cinematográfica · Endurance: la nave diminuta rozando el disco, como en la película | 1 |
| Vista 2 — agujero: inclinada a 30° · Endurance: la nave desde abajo, iluminada por las nubes, contra el arco de Gargantua | 2 |
| Vista 3 — agujero: casi polar, 87° · Endurance: primer plano de la cúpula, con el anillo de fotones detrás | 3 |
| Girar el anillo de la nave (Endurance) | G |
| Quitar / poner las barras de cine (Endurance) | B |
| Pausar / continuar gas, estrellas y giro | Espacio |
| Restablecer cámara | R |
| Salir | Esc |

El skybox se genera al iniciar la variante o la Endurance, y la luz del
entorno de la nave al entrar a la Endurance. Cada uno tarda una fracción de
segundo.

## Rúbrica

Cómo cubre el proyecto cada criterio de evaluación y dónde verlo.

| Criterio | Puntos | Cómo se cumple | Dónde verlo |
| --- | --- | --- | --- |
| Complejidad de la escena | 30 | Trazado de rayos sobre geodésicas de Schwarzschild integradas con RK4; gas volumétrico animado con Doppler y corrimiento gravitacional; nave con SDF de dieciséis tipos de piezas; reflexión y refracción como geodésicas, con hasta tres rebotes guardados en caché; luz del entorno capturada con geodésicas y armónicos esféricos; sombras suaves, oclusión ambiental y texturas con nivel de detalle; render en paralelo con hilos propios | Versión Endurance; [raymarch.rs](src/render/raymarch.rs), [endurance.rs](src/scene/endurance.rs) |
| Atractivo visual | 20 | Composición tomada de la escena de *Interstellar* en que la Endurance roza el disco; disco crema de 5200 K, curva de película, destellos anamórficos, velo de lente, viñeta, grano, barras 2.39:1 y ventanas iluminadas en los módulos | Vistas `1`, `2` y `3` de la Endurance |
| Rotación del diorama y zoom | 20 | La cámara orbita la nave con el mouse o W/A/S/D y se acerca o aleja con la rueda o las flechas, de 0.045 a 12 rs. `G` hace girar el anillo de la nave | [camera.rs](src/camera.rs), [input.rs](src/input.rs), [main.rs](src/main.rs) |
| Materiales | 5 c/u, máx. 25 | Seis materiales, cada uno con su propia textura procedural y sus propios albedo, specular, transparencia y reflectividad, además de rugosidad, índice de refracción, metalicidad y emisión | [material.rs](src/scene/material.rs), tabla de [materiales](#materiales), `--materials` |
| Refracción | 10 | Cúpula de observación de vidrio ($n=1.5$) sobre el tablero de instrumentos del núcleo: ley de Snell al entrar y al salir, reflexión total interna y absorción del vidrio. A través de ella se ven las luces del tablero desplazadas | Vista `3` de la Endurance; `through_glass` en [endurance.rs](src/scene/endurance.rs) |
| Reflexión | 5 | Aislante dorado, aluminio desnudo, mantas plateadas, paneles solares, nervios de la cúpula y vidrio por Fresnel. Los rayos reflejados se integran como geodésicas, así que el casco refleja el disco deformado por la lente | Vistas `2` y `3` de la Endurance; `shade_hit` en [raymarch.rs](src/render/raymarch.rs) |
| Skybox | 20 | Cubemap de seis caras de 768×768 con la Vía Láctea, franjas de polvo, regiones HII y nebulosas, generado al iniciar y deformado por la lente | Variante y Endurance; [skybox.rs](src/scene/skybox.rs) |

## Cómo se forma la imagen

```text
Cámara → geodésicas → emisión y absorción del gas → cielo de fondo
       → historial temporal corto → bloom → exposición y tonemap → ventana

Endurance: luz del entorno capturada con geodésicas desde la nave (una vez)
           geodésica → ¿cuerda toca la nave? → material, luz del entorno,
           sombras → rebotes de reflexión y refracción (nuevas geodésicas)
           → skybox → bloom, velo y destello anamórfico → curva de película
           → barras
```

Las trayectorias y las muestras del volumen se guardan en una caché por vista,
organizada por filas para evitar duplicar todo el gas durante su construcción.
Con la cámara quieta se preparan dos muestras subpíxel y se vuelve a calcular
su iluminación al tiempo actual. El original regenera su textura de gas de
512×256 en cada frame. La variante utiliza un volumen de 384×192×9: azimut,
radio y altura. Dos campos de ruido se guardan durante su ciclo de vida;
cada frame desplaza sus coordenadas con la velocidad orbital local y mezcla
sus valores. El ruido se renueva cuando el campo correspondiente tiene peso
cero. En la variante también se actualiza el cielo.

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
en sentido contrario al fotón recibido. En el código, $b_y$ se almacena en
el campo `lz_over_e`; el eje de rotación empleado es Y.

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
s=\begin{cases}1&\text{original},\\6500/7000&\text{variante},\\5200/7000&\text{Endurance}.\end{cases}
$$

El ajuste de la variante y de la Endurance cambia la temperatura de color
conservando la normalización de brillo. Esta es una representación RGB
aproximada, no una integración espectral completa de la ley de Planck.

### 6. Espesor del disco y gas gris exterior

La transición suave empleada en los bordes es:

$$
\mathcal S(a,b;R)=q^2(3-2q),\qquad
q=\operatorname{clamp}\left(\frac{R-a}{b-a},0,1\right).
$$

El núcleo tiene altura $H_c=0.004R$. La atmósfera usa
$(H_a,\tau_a)=(0.035R,0.035)$ en el original, $(0.028R,0.10)$ en la variante y
$(0.012R,0.10)$ en la Endurance.
El perfil de opacidad térmica es:

$$
W(R)=\mathcal S(3,3.24;R)\,[1-\mathcal S(7.5,13;R)],
$$

$$
D_t=W(R)\left[
\frac{8}{H_c}e^{-y^2/(2H_c^2)}+
\frac{\tau_a}{H_a}e^{-y^2/(2H_a^2)}\right].
$$

La variante añade una envoltura con $H_o=0.028R$ ($0.014R$ en la Endurance):

$$
D_o=\frac{0.30}{H_o}\,
\mathcal S(6,10;R)\,[1-\mathcal S(12,19;R)]\,e^{-y^2/(2H_o^2)}.
$$

$$
c_o=\mathcal S(6,13;R),\qquad
\mathbf k_o=(1-c_o)(1,0.82,0.68)+c_o(0.94,0.96,1),
$$

$$
S_o=0.045\,g^4\mathbf k_o\,e^{-0.22\max(R-6,0)}.
$$

Esta fuente aproxima luz dispersada: pasa de cálida a gris y se atenúa
hacia afuera. Los perfiles se truncan a 3.5 alturas de su atmósfera;
el medio térmico termina en $R=13$ y la envoltura en $R=19$.

En la periferia, una fracción del medio térmico usa también esa fuente gris.
Esta mezcla conserva la opacidad total y hace gradual la transición del borde:

$$
f=\mathcal S(7.5,13;R),\qquad
D_{\mathrm{th}}=(1-f)D_t,\qquad D_{\mathrm{gris}}=D_o+fD_t.
$$

En el original se usan $f=0$ y $D_o=0$. La opacidad y la fuente combinadas son:

$$
D=D_{\mathrm{th}}+D_{\mathrm{gris}},\qquad
S=\frac{D_{\mathrm{th}}S_{\mathrm{th}}+D_{\mathrm{gris}}S_o}{D}.
$$

El enfriamiento visual y la luz dispersada son una aproximación de material,
sin resolver el equilibrio térmico del gas. Se omiten muestras de densidad
despreciable para evitar divisiones por cero.
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
usando el suavizado $f(a)=a^2(3-2a)$. Se suman $n=5$ octavas en el original
y $n=4$ en la variante:

$$
\operatorname{fBm}(\mathbf q)=\frac12+\frac12
\frac{\sum_{k=0}^{n-1}2^{-(k+1)}N(2^k\mathbf q)}
{\sum_{k=0}^{n-1}2^{-(k+1)}}.
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
m=1+0.96\left[e^{\kappa(F-0.5)}-1\right],\qquad
\kappa=\begin{cases}8&\text{original},\\11&\text{variante}.\end{cases}
$$

En la variante, la altura normalizada $h=y/H_o$ modifica el dominio de ruido
de cada campo:

$$
A=3.2+0.5h,\qquad \theta_j=\phi_j+0.18h,\qquad
\mathbf q_j=1.5(A\cos\theta_j,\ A\sin\theta_j,\ 1.3R)+s_j(1,1,1).
$$

$s_j$ es la semilla espacial del campo durante su ciclo. Así el ruido cambia
con la altura y forma filamentos con cizalla. La variante almacena cada campo
en una rejilla cilíndrica, interpola el desplazamiento angular y consulta el
volumen final mediante interpolación trilineal. El original consulta su campo
mediante interpolación bilineal.

Los campos se renuevan cuando su peso es cero, evitando reinicios visibles.
La rotación depende del radio: las partes interiores se mueven más rápido.
El factor 8 acelera el tiempo de animación; el Doppler sigue usando
la velocidad orbital del modelo físico. La Endurance usa los mismos campos de
gas que la variante.

Implementación: [ruido](src/math/noise.rs) y [textura del disco](src/scene/disk.rs).

### 9. Estrellas y movimiento aparente del entorno

El cielo se consulta con la dirección de salida de la geodésica,
$\mathbf d_{\mathrm{esc}}$. En la variante y en la Endurance se transforma con:

$$
\mathbf d_{\mathrm{cielo}}(t)=
R_z(0.23)R_y(0.012t)R_z(-0.23)\mathbf d_{\mathrm{esc}}.
$$

Los ángulos están en radianes. Esta deriva angular representa movimiento
relativo del fondo, mientras la métrica del agujero permanece estática.
El estiramiento de las estrellas procede del mapa de geodésicas. El skybox se
consulta con esa misma dirección, así que la Vía Láctea deriva junto con las
estrellas.

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
C_b=C_h+\beta\sum_{k=0}^{5}\frac{0.72^k}{\sum_{j=0}^{5}0.72^j}B_k,
$$

$$
\beta=\begin{cases}0.38&\text{original},\\0.24&\text{variante},\\0.20&\text{Endurance}.\end{cases}
$$

Aquí $B_k$ representa cada nivel ya desenfocado y reescalado. Finalmente:

$$
C_e=\max(5C_b,0),\qquad
C_{\mathrm{pantalla}}=
\left(\frac{C_e}{1+\max(C_{e,R},C_{e,G},C_{e,B})}\right)^{1/2.2}.
$$

La división usa un factor común para RGB, preservando las proporciones
de color antes de aplicar gamma. El halo y la exposición representan la
presentación de la cámara; no modifican las trayectorias de luz. La Endurance
expone 1.3 en lugar de 5, así que su umbral de bloom es $0.6/1.3$, y usa la
curva de película descrita en su sección.

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
Alrededor de la nave, el exponente es $0.11\delta$ y el rango va de 0.045 a
12 rs.

Implementación: [cámara](src/camera.rs) y [entrada](src/input.rs).

## Versión Endurance

La nave vuela a 7.6 radios de Schwarzschild del eje, a 0.3 rs sobre el plano
del disco, rozando el tope del mar de nubes y en dirección al agujero. Su
anillo mide 0.02 rs: la nave es diminuta frente a Gargantua, como en la toma
de la película en que la Endurance pasa sobre el disco. El zoom va de 12 rs a
0.045 rs de la nave, más de 260 veces: desde el plano completo de Gargantua,
con la nave como un punto, hasta los detalles del casco. La cámara no baja de
0.08 rs sobre el plano, porque más abajo entraría al gas opaco. Tampoco entra
a la esfera de 2.5 rs alrededor del agujero: si la órbita la cruzaría, se
detiene en su borde y recupera la distancia pedida al girar hacia otro lado.

En esta versión el gas es más compacto que en la variante (atmósfera de altura
$0.012R$ y envoltura de $0.014R$, en vez de $0.028R$): por encima del tope de
las nubes queda espacio despejado y la sombra se ve oscura. El disco usa una
temperatura de color de pico de 5200 K.

### Materiales

| Material | Textura procedural | Albedo | Specular | Rugosidad | Transparencia | Reflectividad $F_0$ | $n$ | Metal | Emisión | Dónde está |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Casco de aluminio pintado | Paneles de largo irregular: pintura térmica, aluminio desnudo, pintura gris, mantas plateadas acolchadas y mantas negras de kapton; juntas, hollín y ventanas | (0.60, 0.60, 0.58) | 0.5 | 0.42 | 0 | 0.06 | — | 0 | Ventanas de las cabinas | Módulos, túneles, radios, tanques, equipos y lomo del Ranger |
| Aislante dorado | Lámina multicapa arrugada con cintas de unión | (1.00, 0.76, 0.34) | 1.0 | 0.20 | 0 | 0.85 | — | 1 | — | Banda de los módulos habitables |
| Panel solar | Celdas con barras colectoras y separaciones claras, bajo vidrio | (0.03, 0.05, 0.14) | 1.0 | 0.06 | 0 | 0.05 | — | 0 | — | Alas de los módulos de energía |
| Vidrio de la cúpula | Nervios metálicos y manchas leves | (0.92, 0.96, 1.00) | 1.0 | 0.02 | 0.92 | 0.04 | 1.5 | 0 | — | Cúpula de observación |
| Losetas térmicas | Losetas negras con reemplazos claros; tablero con luces de estado | (0.055, 0.055, 0.06) | 0.3 | 0.70 | 0 | 0.03 | — | 0 | Luces del tablero | Núcleo, tablero bajo la cúpula y panza del Ranger |
| Tobera del motor | Metal oxidado por calor: paja, bronce, púrpura y azul | (0.30, 0.26, 0.23) | 0.8 | 0.32 | 0 | 0.60 | — | 0.85 | Plasma del escape | Dos toberas por módulo de motor |

La textura no es un tinte fijo: en cada punto decide el albedo, la rugosidad,
la metalicidad, el brillo especular, la reflectividad, la transparencia, la
emisión y una altura cuyo gradiente inclina la normal (relieve). La mezcla de
pintura, metal desnudo y mantas del casco es lo que da el aspecto de nave real:
los metales reflejan el cielo negro arriba y el disco abajo.

Cada textura recibe el tamaño del píxel en el punto de impacto,
$\ell=\text{distancia}\cdot\theta_{\text{píxel}}$, y apaga los detalles más
chicos que medio píxel con $\mathcal S(0.25,2;\text{tamaño}/\ell)$,
reemplazándolos por su promedio. Así una nave lejana no parpadea y una cercana
muestra cada junta. Para exportar una muestra de cada textura e imprimir la
tabla de parámetros:

```sh
cargo run --release -- --materials artifacts/materiales
```

### Geometría y trazado de la nave

La nave se describe con funciones de distancia con signo en un marco local
donde el anillo mide 1 y el eje Y apunta en la dirección de vuelo:

- Doce módulos (cajas con bordes redondeados) de tres tipos: habitables con
  aislante dorado, ventanas y caja de equipo; de motor con dos toberas y dos
  tanques de propelente; y de energía con un ala de paneles solares sobre un
  mástil. Todos llevan dos tuberías por la cara interior.
- Túneles cilíndricos con brida entre módulos, cuatro radios dobles hacia el
  núcleo, el núcleo con dos bridas y el collar de acople.
- La cúpula de vidrio atrás del núcleo y el transbordador Ranger acoplado
  adelante: fuselaje con nariz en punta, alas delta y deriva, como cajas
  recortadas por planos.

Módulos, túneles y radios se evalúan por repetición angular: solo la celda
propia y la vecina. Cada región (anillo, radios y núcleo) tiene además una cota
inferior de distancia en el plano $(R,y)$: si ya hay algo más cerca, la región
no se evalúa. Una prueba verifica que esas cotas nunca sobreestimen la
distancia.

Dentro de la esfera envolvente de la nave, cada paso de la geodésica se trata
como una cuerda recta $\mathbf a\to\mathbf b$ y se recorre con sphere tracing:

$$
t_{k+1}=t_k+d\big(\mathbf a+t_k\hat{\mathbf u}\big),\qquad
\text{impacto si } d<2.5\times10^{-4}\ \text{(unidades locales)} .
$$

La nave mide centésimas de rs, cerca del límite de precisión de un `f32` a
8 rs del agujero. Por eso la marcha se hace en coordenadas relativas a su
centro: la resta grande se hace una vez por cuerda y después los números son
del tamaño de la nave. La normal es el gradiente de $d$ con cuatro muestras en
tetraedro.

### Luz del entorno

La luz no se coloca a mano. Al entrar a la versión se trazan 2048 geodésicas
desde el centro de la nave en direcciones de Fibonacci, con el mismo
integrador, el mismo gas y el mismo Doppler que la cámara. El resultado es lo
que vería un observador en la nave: el mar de nubes debajo, el lado del disco
que se acerca y su imagen curvada sobre la sombra. Esas radiancias $L_i$ se
dividen en dos partes:

- **Luces clave**: las muestras se agrupan en 48 zonas y las tres más
  brillantes se vuelven luces direccionales, con irradiancia
  $\mathbf E_k=\sum_{i\in k}L_i\,\Delta\omega$ y un tamaño angular sacado de
  la concentración de sus direcciones, que fija la penumbra de su sombra.
- **Armónicos esféricos de orden 2** con el resto (Ramamoorthi y Hanrahan):

$$
c_{lm}=\sum_i L_i\,Y_{lm}(\hat{\boldsymbol\omega}_i)\,\Delta\omega,\qquad
\mathbf E(\mathbf n)=\sum_{l\le2,\,m}\hat A_l\,c_{lm}\,Y_{lm}(\mathbf n),\qquad
\hat A_0=\pi,\ \hat A_1=\tfrac{2\pi}{3},\ \hat A_2=\tfrac{\pi}{4}.
$$

Con una luz uniforme $L$ esto da exactamente $\mathbf E=\pi L$ para cualquier
normal, y una prueba lo verifica. El sombreado combina difuso, brillos de
microfacetas GGX y el reflejo borroso de las superficies rugosas:

$$
\mathbf L=\mathbf L_e
+k_d\,\frac{\boldsymbol\rho}{\pi}\Big(\mathbf E(\mathbf n)\,A+\sum_k \mathbf E_k\,(\mathbf n\cdot\mathbf l_k)\,V_k\Big)
+s\sum_k \mathbf E_k V_k\,\frac{D\,G\,\mathbf F}{4\,(\mathbf n\cdot\mathbf v)}
+\mathbf F_v\,\mathcal S(0.12,0.55;r)\,\frac{\mathbf E_{\text{total}}(\mathbf r)}{\pi}\,A ,
$$

$$
D=\frac{\alpha^2}{\pi\big((\mathbf n\cdot\mathbf h)^2(\alpha^2-1)+1\big)^2},\qquad
\alpha=r^2,\qquad
G=G_1(\mathbf n\cdot\mathbf v)\,G_1(\mathbf n\cdot\mathbf l),\quad
G_1(x)=\frac{x}{x(1-k)+k},\quad k=\frac{(r+1)^2}{8}.
$$

$r$ es la rugosidad, ensanchada por el tamaño angular de cada luz para que las
superficies lisas no generen brillos imposiblemente chicos, y $s$ el specular
del material. La visibilidad $V_k$ es una sombra suave por marcha de
distancias, $V=\mathcal S\big(\min_j \kappa h_j/t_j\big)$, y la oclusión
ambiental $A=1-3.2\sum_{i=1}^{5}0.62^{\,i-1}\big(h_i-d(\mathbf p+h_i\mathbf n)\big)$.
El relieve inclina la normal en el plano tangente:
$\mathbf n'=\operatorname{normalize}\big(\mathbf n-s(\partial_u h\,\mathbf t_u+\partial_v h\,\mathbf t_v)\big)$.

### Reflexión y refracción

$$
\mathbf r=\mathbf d-2(\mathbf d\cdot\mathbf n)\mathbf n,\qquad
\mathbf t=\eta\,\mathbf d+\big(\eta\cos\theta_i-\cos\theta_t\big)\mathbf n,\qquad
\cos\theta_t=\sqrt{1-\eta^2(1-\cos^2\theta_i)} .
$$

Si la raíz es imaginaria hay reflexión total interna. La reflectancia sigue
la aproximación de Schlick y reparte la energía entre los tres caminos:

$$
\mathbf F_v=\boldsymbol\tau\big(F_0+(1-F_0)(1-\cos\theta_i)^5\big),\qquad
k_r=\max\mathbf F_v,\quad k_t=(1-k_r)\,T,\quad k_d=(1-k_r)(1-T)(1-m).
$$

En los metales ($m=1$) el reflejo toma el tono del albedo, $\boldsymbol\tau$.
Dentro del vidrio la luz se atenúa según Beer-Lambert,
$\mathbf A=e^{-\boldsymbol\sigma\ell}$ con $\boldsymbol\sigma=(0.9,0.35,0.25)$
por unidad local, lo que tiñe de cian los bordes gruesos. El color final de un
impacto es:

$$
\mathbf C=\mathbf C_{\text{local}}+\mathbf F_v\big(1-\mathcal S(0.12,0.55;r)\big)\,\mathbf C(\mathbf r)+k_t\,\mathbf A\,\mathbf C(\mathbf t).
$$

Se siguen hasta tres rebotes. Los rayos hijos se guardan en la caché junto con
sus muestras de gas y se vuelven a sombrear en cada frame: con la cámara
quieta, el reflejo del disco en el casco sigue animado.

### Skybox

Cada texel del cubemap se llena una sola vez con la dirección de su centro. La
cara es el eje dominante y las coordenadas son las otras dos componentes
divididas por él. El cielo se describe en coordenadas galácticas,
$b=\arcsin(\hat{\mathbf d}\cdot\mathbf P)$ y
$l=\operatorname{atan2}(\hat{\mathbf d}\cdot\mathbf S,\hat{\mathbf d}\cdot\mathbf C)$:
una banda $e^{-((b-w(l))/\sigma(l))^2}$ que ondula y se ensancha hacia el bulbo,
nubes de fBm con deformación de dominio, franjas de polvo oscuro, regiones HII
rosadas y nebulosas tenues. El bulbo queda arriba a la izquierda de la vista de
la variante, y la lente dobla la banda en un arco alrededor de la sombra. Los
texels se guardan en RGBE de 32 bits: tres mantisas de 8 bits y un exponente
común, como el formato de Radiance. Las estrellas puntuales se evalúan aparte,
por rayo, para que sigan nítidas.

### Presentación de película

La curva ACES de Narkowicz, $f(x)=\frac{x(2.51x+0.03)}{x(2.43x+0.59)+0.14}$,
se aplica al canal máximo para conservar el tono del disco. Solo los brillos
extremos se mezclan con $f$ por canal y se vuelven blancos, y la saturación
baja a 0.72, como en una emulsión:

$$
\mathbf C=\operatorname{lerp}\Big(\mathbf C_e\frac{f(M)}{M},\;f(\mathbf C_e),\;0.75\,\mathcal S\big(0.7,1;f(M)\big)\Big),
\qquad M=\max(\mathbf C_e).
$$

Después hay un viraje leve (sombras hacia el cian), un velo de lente con los
dos niveles más anchos del bloom, viñeta, grano por frame y barras 2.39:1. Las
filas detrás de las barras no se trazan. El destello anamórfico estira
horizontalmente los puntos más brillantes con un filtro exponencial en ambos
sentidos, de costo lineal: $y_n=a\,y_{n-1}+x_n$, con $a=e^{-1/(0.12W)}$ y
normalizado por $(1-a)/(1+a)$.

## Validación y capturas

```sh
cargo test --release
cargo clippy --all-targets -- -D warnings
cargo run --release -- --verify
```

Las 25 pruebas cubren continuidad del ruido y del flujo, sentido de advección,
Doppler, integración de opacidad, conservación del color, caché, animación
del cielo, rayos capturados y desvanecimiento del gas gris. Verifican el detalle
vertical del volumen, su continuidad al renovar campos, saltar en el tiempo y
alternar versiones. También cubren la orientación y composición de las rotaciones
propias, la normalización de vectores y el procesamiento paralelo, incluidos
bloques incompletos, trabajos anidados y finalización de los préstamos cuando
hay un fallo en los hilos.

Otras 18 pruebas, 43 en total, cubren la Endurance:
- **Versiones y materiales:** el ciclo de `V` por las tres versiones, y que los
  seis materiales tengan parámetros válidos y texturas distintas, que varían de
  cerca y se aquietan de lejos.
- **Óptica:** la reflexión, la ley de Snell, la reflexión total interna, los
  límites de Fresnel y la normalización de la distribución GGX.
- **Nave:** que la geometría quepa en su esfera envolvente con todas sus piezas,
  que las cotas por región nunca sobreestimen la distancia, que los rayos peguen
  en la nave y produzcan reflejos, y que la luz que entra a la cúpula llegue al
  tablero. También que la nave vuele sobre el gas, que la cámara no baje a él
  y que, al alejarse, nunca entre a la esfera de exclusión del agujero.
- **Luz del entorno:** que un cielo uniforme dé $\pi L$ desde cualquier lado y
  que un piso luminoso ilumine desde abajo y se vuelva una luz clave.
- **Cielo y presentación:** la ida y vuelta del cubemap, la precisión del RGBE,
  la continuidad del cielo entre caras, la curva de película y las barras de
  cine.

La verificación analítica comprueba $b_c$, conservación de energía y momento
angular, la órbita de fotones en $r=1.5$ y la deflexión en campo débil:

$$
\alpha(b)\approx\frac{4M}{b}+\frac{15\pi}{4}\left(\frac{M}{b}\right)^2,
\qquad M/b\ll1.
$$

Esta expresión sirve de referencia para validar la deflexión de los rayos.
`--verify` informa los valores medidos, las referencias analíticas y sus
errores; devuelve un código de error si alguna comprobación falla.

Para medir arrastre real y cámara quieta, guardando el último frame:

```sh
cargo run --release -- --probe 30 frame.ppm 2 21.5 2
cargo run --release -- --probe 30 original.ppm 2 21.5 2 --original
```

Argumentos de `--probe`: frames, salida PPM, elevación en grados, distancia
en radios de Schwarzschild, tiempo inicial en segundos y, opcionalmente, el
yaw de la cámara en radianes. Con `--endurance` la distancia se mide desde la
nave:

```sh
cargo run --release -- --probe 12 heroe.ppm --endurance
cargo run --release -- --probe 12 desde-abajo.ppm -14 0.17 0 1.0 --endurance
```

Para exportar una secuencia a 30 muestras por segundo o una captura individual:

```sh
cargo run --release -- --sequence artifacts/secuencia 90 2 21.5 0 960 540
cargo run --release -- --sequence artifacts/captura 1 30 36 2 1920 1080
```

Argumentos de `--sequence`: directorio, frames, elevación, distancia, tiempo
inicial, ancho y alto. Se puede añadir `--original` o `--endurance` al final
para elegir la versión, y `--spin` para que el anillo de la Endurance gire
durante la secuencia. El directorio de salida se crea automáticamente.

```sh
cargo run --release -- --sequence artifacts/endurance 150 -12 0.075 0 1920 1080 --endurance --spin
```

Los PPM contienen el render sin compresión. Las exportaciones locales en
`artifacts/` se excluyen de Git. Los FPS de reproducción de un
GIF o video exportado no son una medición del rendimiento interactivo.

## Rendimiento

`--probe` mide por separado el arrastre real a escala 0.33 y la cámara quieta
a resolución completa. Informa tanto el promedio total como el promedio
después de los primeros frames de preparación.

Preparar las trayectorias cuesta más que sombrear una vista ya calculada y
se repite al mover la cámara o cambiar de versión. La caché conserva dos muestras
por píxel y su memoria crece con la resolución y la cantidad de gas atravesado.
La variante reutiliza los campos de ruido y renueva uno cada 2.5 segundos de
animación; esa renovación tiene un costo adicional en ese frame.

Los resultados dependen del CPU, su temperatura, la memoria disponible,
la resolución y la vista. El volumen con detalle vertical, la envoltura
adicional y el cielo animado aumentan el costo de la variante.

En la Endurance, los rayos que pegan en la nave lanzan rebotes y sombras, y
eso encarece la preparación de la vista. Las barras de cine ahorran una cuarta
parte de las filas, y el gas compacto deja menos muestras por rayo. Por eso,
una vez preparada la vista, su costo por frame es menor que el de la variante.
En un i5-12500H (16 hilos), la vista `1` mantiene unos 25 FPS quieta y 40 FPS
al arrastrar; la vista `2`, unos 32 y 35 FPS. Mientras el anillo gira con `G`,
la vista cambia en cada frame y se traza a la escala reducida del arrastre.
La variante conserva unos 10 FPS quieta y 17 al arrastrar con su fondo nuevo.

La versión original conserva el rendimiento y la imagen que tenía antes de
agregar la Endurance, píxel por píxel: el integrador se compila en dos copias,
y la que no conoce la nave no carga con la recursión de los rebotes.

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
- La escala de la Endurance es cinematográfica, no física. Dentro de su esfera
  envolvente los rayos se aproximan por cuerdas rectas; la curvatura en ese
  tramo desplaza el impacto en mucho menos que el tamaño de un panel.
- La luz del entorno se captura una vez, en el centro de la nave y con el gas
  de ese instante: no varía de un extremo a otro de la nave ni sigue la
  turbulencia. Tres luces clave y armónicos de orden 2 la resumen; los reflejos
  de las superficies lisas sí son rayos completos. Dentro del vidrio, la luz
  viaja en línea recta.
- La nave queda fija en la caché de rayos. Al girar el anillo se vuelve a
  trazar la vista; no se interpola entre posiciones.

## Estructura del proyecto

| Archivo | Responsabilidad |
| --- | --- |
| [main.rs](src/main.rs) | Ventana, cambio con V, giro con G, pausa, benchmark y exportación |
| [version.rs](src/version.rs) | Las tres versiones y su orden |
| [config.rs](src/config.rs) | Constantes físicas, apariencia de las tres versiones, nave y cámara |
| [camera.rs](src/camera.rs), [input.rs](src/input.rs) | Proyección, órbita, zoom, vistas por versión y controles |
| [relativity.rs](src/scene/relativity.rs), [blackhole.rs](src/scene/blackhole.rs) | Frecuencias, invariantes y geodésicas |
| [disk.rs](src/scene/disk.rs) | Forma del gas de cada versión, emisión, absorción, enfriamiento exterior y campos de gas con advección |
| [stars.rs](src/scene/stars.rs) | Cielo original y fondo animado de la variante y la Endurance |
| [skybox.rs](src/scene/skybox.rs) | Cubemap de la Vía Láctea en RGBE |
| [endurance.rs](src/scene/endurance.rs) | Geometría SDF de la nave, sombreado, sombras y vidrio |
| [material.rs](src/scene/material.rs) | Los seis materiales, sus texturas con nivel de detalle, Snell, Fresnel, GGX y reflexión |
| [lighting.rs](src/scene/lighting.rs) | Luz del entorno de la nave: luces clave y armónicos esféricos |
| [noise.rs](src/math/noise.rs) | Ruido continuo y fBm |
| [vector.rs](src/math/vector.rs) | Vectores, matrices y operadores propios |
| [parallel.rs](src/parallel.rs) | Reparto de bloques con hilos de Rust estándar |
| [raymarch.rs](src/render/raymarch.rs) | Muestreo del volumen, caché de trayectorias, impactos, rebotes y captura de la luz del entorno |
| [render/mod.rs](src/render/mod.rs) | Selección de versión y etapas del render |
| [accumulate.rs](src/render/accumulate.rs), [bloom.rs](src/render/bloom.rs), [tonemap.rs](src/render/tonemap.rs) | Historial, halo, velo de lente, destello anamórfico y presentación HDR y de película |
| [framebuffer.rs](src/render/framebuffer.rs) | Buffers, reescalado y presentación |
| [verify.rs](src/verify.rs) | Comprobaciones analíticas |

Los parámetros `PREVIEW_*`, `OUTER_GAS_*` y `SKY_DRIFT_SPEED` controlan
la variante; los `SHIP_*`, `ENDURANCE_*`, `SKYBOX_*`, `FILM_*`, `GLARE_*` y
`STREAK_*`, la Endurance. Las alturas del gas de cada versión se agrupan en
`disk::Medium`. La selección pasa por `Renderer::set_version` y forma parte de
la clave de caché, junto con el giro de la nave; esto permite comparar cambios
visuales sin mezclar muestras o imágenes de distintas versiones.

## Referencias

- [NASA: visualización de un agujero negro y su disco](https://www.nasa.gov/universe/nasa-visualization-shows-a-black-holes-warped-world/), referencia visual de lente gravitacional y asimetría de brillo.
- [James, von Tunzelmann, Franklin y Thorne (2015)](https://arxiv.org/abs/1502.03808), descripción del render de *Interstellar* mediante haces de luz en Kerr. Este proyecto usa rayos individuales en Schwarzschild.
