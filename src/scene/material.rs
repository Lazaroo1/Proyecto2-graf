//! Materiales de la Endurance y su optica.
//!
//! Cada material tiene su propia textura procedural y sus propios parametros
//! de albedo, specular, transparencia y reflectividad, ademas de rugosidad,
//! metalicidad, indice de refraccion y emision. La textura no es un tinte
//! encima de un color fijo: en cada punto decide el albedo, la rugosidad, la
//! metalicidad, la reflectividad, la transparencia, la emision y el relieve.
//!
//! # Nivel de detalle
//!
//! La nave puede ocupar unos pocos pixeles o la pantalla entera. Cada textura
//! recibe el tamano del pixel en el punto (`lod`, en unidades de uv) y apaga
//! los detalles mas chicos que un pixel, reemplazandolos por su promedio. Sin
//! eso, juntas y remaches de una nave lejana parpadean como ruido.
//!
//! # Optica
//!
//! - **Reflexion**: `r = d - 2 (d.n) n`. El rayo reflejado vuelve a integrarse
//!   como geodesica, asi que el casco refleja el disco ya deformado por la lente.
//! - **Refraccion**: ley de Snell, `n1 sin(t1) = n2 sin(t2)`, con reflexion
//!   total interna cuando no hay solucion.
//! - **Fresnel**: aproximacion de Schlick, `F = F0 + (1 - F0)(1 - cos)^5`.
//! - **Brillos**: microfacetas GGX con sombreado de Smith-Schlick.

use crate::math::noise::NoiseTable;
use crate::math::{curves, Vec2, Vec3};
use std::f32::consts::PI;

/// Indice en [`MATERIALS`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialId {
    Hull = 0,
    Gold = 1,
    Solar = 2,
    Glass = 3,
    Ceramic = 4,
    Nozzle = 5,
}

impl MaterialId {
    pub fn material(self) -> &'static Material {
        &MATERIALS[self as usize]
    }
}

/// Patron procedural propio de cada material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Texture {
    /// Pintura termica blanca, paneles de aluminio desnudo y mantas acolchadas.
    HullPanels,
    /// Lamina de aislamiento multicapa arrugada.
    GoldFoil,
    /// Celdas fotovoltaicas con barras colectoras bajo vidrio.
    SolarCells,
    /// Vidrio con marco de nervios metalicos y manchas leves.
    DomeGlass,
    /// Losetas del escudo termico.
    HeatTiles,
    /// Metal quemado con el degradado de calor de una tobera.
    BurntMetal,
}

impl Texture {
    /// Paso de la diferencia finita y escala del relieve. La cupula usa uv en
    /// radianes; las demas, unidades locales de la nave.
    pub fn bump(self) -> (f32, f32) {
        match self {
            Texture::HullPanels => (0.0008, 0.0009),
            Texture::GoldFoil => (0.0008, 0.0018),
            Texture::SolarCells => (0.0005, 0.0004),
            Texture::DomeGlass => (0.004, 0.002),
            Texture::HeatTiles => (0.0005, 0.0006),
            Texture::BurntMetal => (0.002, 0.0015),
        }
    }
}

/// Parametros de un material. Los rangos son fracciones en `[0, 1]`, salvo
/// el indice de refraccion.
#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub name: &'static str,
    pub albedo: Vec3,
    /// Intensidad de los brillos especulares de las luces.
    pub specular: f32,
    /// Rugosidad de las microfacetas: 0 es un espejo, 1 un material mate.
    pub roughness: f32,
    /// Fraccion de la luz que atraviesa la superficie.
    pub transparency: f32,
    /// Reflectancia a incidencia normal (F0 de Schlick).
    pub reflectivity: f32,
    /// Indice de refraccion. Solo importa si hay transparencia.
    pub ior: f32,
    /// 1 para metales: el reflejo toma el color del albedo y no hay difuso.
    pub metalness: f32,
    /// Luz propia, en las mismas unidades HDR que el disco.
    pub emission: Vec3,
    pub texture: Texture,
}

pub static MATERIALS: [Material; 6] = [
    Material {
        name: "Casco de aluminio pintado",
        albedo: Vec3::new(0.6, 0.6, 0.58),
        specular: 0.5,
        roughness: 0.42,
        transparency: 0.0,
        reflectivity: 0.06,
        ior: 1.0,
        metalness: 0.0,
        // Luz calida de las cabinas, solo donde la textura dibuja ventanas.
        emission: Vec3::new(2.4, 1.6, 0.85),
        texture: Texture::HullPanels,
    },
    Material {
        name: "Aislante dorado",
        albedo: Vec3::new(1.0, 0.76, 0.34),
        specular: 1.0,
        roughness: 0.2,
        transparency: 0.0,
        reflectivity: 0.85,
        ior: 1.0,
        metalness: 1.0,
        emission: Vec3::ZERO,
        texture: Texture::GoldFoil,
    },
    Material {
        name: "Panel solar",
        albedo: Vec3::new(0.03, 0.05, 0.14),
        specular: 1.0,
        roughness: 0.06,
        transparency: 0.0,
        reflectivity: 0.05,
        ior: 1.0,
        metalness: 0.0,
        emission: Vec3::ZERO,
        texture: Texture::SolarCells,
    },
    Material {
        name: "Vidrio de la cupula",
        albedo: Vec3::new(0.92, 0.96, 1.0),
        specular: 1.0,
        roughness: 0.02,
        transparency: 0.92,
        reflectivity: 0.04,
        ior: 1.5,
        metalness: 0.0,
        emission: Vec3::ZERO,
        texture: Texture::DomeGlass,
    },
    Material {
        name: "Losetas termicas",
        albedo: Vec3::new(0.055, 0.055, 0.06),
        specular: 0.3,
        roughness: 0.7,
        transparency: 0.0,
        reflectivity: 0.03,
        ior: 1.0,
        metalness: 0.0,
        emission: Vec3::new(1.6, 1.0, 0.45),
        texture: Texture::HeatTiles,
    },
    Material {
        name: "Tobera del motor",
        albedo: Vec3::new(0.3, 0.26, 0.23),
        specular: 0.8,
        roughness: 0.32,
        transparency: 0.0,
        reflectivity: 0.6,
        ior: 1.0,
        metalness: 0.85,
        emission: Vec3::new(0.5, 0.9, 2.2),
        texture: Texture::BurntMetal,
    },
];

/// Zona especial de una superficie que su textura dibuja distinto.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Region {
    #[default]
    Plain,
    /// Tablero de instrumentos bajo la cupula: luces de estado.
    Instruments,
    /// Cara de un modulo habitable: ventanas con luz interior.
    Windows,
    /// Boca de la tobera: el plasma del escape.
    Exhaust,
}

/// Lo que la textura decide en un punto. `albedo`, `roughness` y `metalness`
/// son valores finales; los demas multiplican el parametro del material.
#[derive(Clone, Copy, Debug)]
pub struct TextureSample {
    pub albedo: Vec3,
    pub roughness: f32,
    pub metalness: f32,
    pub specular: f32,
    pub reflectivity: f32,
    pub transparency: f32,
    pub emission: f32,
    /// Altura del relieve; su gradiente perturba la normal.
    pub height: f32,
}

impl TextureSample {
    fn lerp(self, other: Self, t: f32) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * t;
        Self {
            albedo: self.albedo.lerp(other.albedo, t),
            roughness: mix(self.roughness, other.roughness),
            metalness: mix(self.metalness, other.metalness),
            specular: mix(self.specular, other.specular),
            reflectivity: mix(self.reflectivity, other.reflectivity),
            transparency: mix(self.transparency, other.transparency),
            emission: mix(self.emission, other.emission),
            height: mix(self.height, other.height),
        }
    }
}

/// Cuanto de un detalle de tamano `feature` se ve con pixeles de tamano
/// `lod`: 1 si ocupa al menos dos pixeles, 0 si cae debajo de medio.
#[inline]
fn detail(feature: f32, lod: f32) -> f32 {
    curves::smoothstep(0.25, 2.0, feature / lod.max(1.0e-7))
}

impl Material {
    /// Evalua la textura en coordenadas de superficie, en unidades locales de
    /// la nave (o radianes para la cupula). `lod` es el tamano de un pixel
    /// en esas mismas unidades.
    pub fn sample(&self, noise: &NoiseTable, uv: Vec2, region: Region, lod: f32) -> TextureSample {
        let base = TextureSample {
            albedo: self.albedo,
            roughness: self.roughness,
            metalness: self.metalness,
            specular: 1.0,
            reflectivity: 1.0,
            transparency: 1.0,
            emission: 0.0,
            height: 0.0,
        };
        match self.texture {
            Texture::HullPanels => {
                let panels = hull_panels(base, noise, uv, lod);
                if region == Region::Windows {
                    windows(panels, uv, lod)
                } else {
                    panels
                }
            }
            Texture::GoldFoil => gold_foil(base, noise, uv, lod),
            Texture::SolarCells => solar_cells(base, uv, lod),
            Texture::DomeGlass => dome_glass(base, noise, uv, lod),
            Texture::HeatTiles => heat_tiles(base, uv, region, lod),
            Texture::BurntMetal => burnt_metal(base, noise, uv, region),
        }
    }
}

fn hull_panels(base: TextureSample, noise: &NoiseTable, uv: Vec2, lod: f32) -> TextureSample {
    let size = Vec2::new(1.0 / 14.0, 1.0 / 9.0);
    // Filas alternas desplazadas: paneles de largo irregular.
    let mut p = Vec2::new(uv.x / size.x, uv.y / size.y);
    let row = p.y.floor();
    p.x += 0.5 * (hash(0, row as i32, 11) + row * 0.37);
    let cell = p.floor();
    let f = p - cell;
    let id = hash(cell.x as i32, cell.y as i32, 3);
    let border = f.x.min(1.0 - f.x).min(f.y).min(1.0 - f.y);
    let seam = (1.0 - curves::smoothstep(0.0, 0.025, border)) * detail(0.002, lod);
    let grime = noise.fbm(Vec3::new(uv.x * 9.0, uv.y * 9.0, 7.3), Vec3::ZERO, 4);
    let soot = curves::smoothstep(0.5, 0.85, grime);

    // Tipo de panel. Las naves reales mezclan pintura termica blanca,
    // aluminio desnudo, pintura gris, mantas plateadas y mantas negras. Los
    // metales reflejan el cielo negro arriba y el disco abajo: eso da el
    // contraste de una nave real y no de una maqueta pintada pareja.
    let mut s = base;
    if id < 0.2 {
        s.albedo = Vec3::new(0.62, 0.63, 0.66);
        s.metalness = 0.9;
        s.roughness = 0.24;
        s.reflectivity = 10.0;
    } else if id > 0.8 {
        // Manta plateada: costuras en rejilla y bolsas infladas entre ellas.
        let quilt = Vec2::new((uv.x * 40.0).fract() - 0.5, (uv.y * 40.0).fract() - 0.5);
        let stitch = 1.0 - curves::smoothstep(0.38, 0.5, quilt.x.abs().max(quilt.y.abs()));
        let wrinkle = 1.0 - noise.value_noise_3d(Vec3::new(uv.x * 160.0, uv.y * 160.0, 2.1)).abs();
        let fine = detail(0.006, lod);
        s.albedo = Vec3::new(0.8, 0.8, 0.79) * (0.85 + 0.15 * wrinkle * fine);
        s.metalness = 1.0;
        s.roughness = 0.18 + 0.12 * (1.0 - fine);
        s.reflectivity = 12.0;
        s.height = (stitch * 0.8 + wrinkle * 0.5) * fine;
    } else if id < 0.3 {
        s.albedo = base.albedo * 0.45;
        s.roughness = 0.55;
    } else if id < 0.36 {
        // Manta negra de kapton.
        s.albedo = Vec3::splat(0.035);
        s.roughness = 0.5;
        s.reflectivity = 0.8;
    } else {
        s.albedo = base.albedo * (0.93 + 0.12 * id);
    }
    s.albedo = s.albedo * (1.0 - 0.18 * soot) * (1.0 - 0.45 * seam);
    s.roughness = (s.roughness + 0.2 * soot + 0.3 * seam).min(1.0);
    s.reflectivity *= 1.0 - 0.8 * seam;
    s.height += -seam + 0.1 * (id - 0.5);

    // Lejos, cada panel ocupa menos que un pixel: se ve el promedio de la mezcla.
    let average = TextureSample {
        albedo: base.albedo * 0.8 * (1.0 - 0.18 * soot),
        roughness: 0.36,
        metalness: 0.38,
        specular: 1.0,
        reflectivity: 4.5,
        transparency: 1.0,
        emission: 0.0,
        height: 0.0,
    };
    average.lerp(s, detail(size.x, lod))
}

/// Hilera de ventanas a cada lado del aislante dorado. `uv` en coordenadas del
/// modulo: x tangencial y z radial, con el centro del modulo en el origen.
fn windows(mut s: TextureSample, uv: Vec2, lod: f32) -> TextureSample {
    let along = uv.x.abs() - 0.065;
    if !(0.0..0.09).contains(&along) || uv.y.abs() > 0.04 {
        return s;
    }
    let slot = (along / 0.03).floor();
    let local = Vec2::new(along - (slot + 0.5) * 0.03, uv.y);
    // Rectangulo redondeado: distancia al borde de la ventana.
    let q = Vec2::new(local.x.abs() - 0.007, local.y.abs() - 0.024);
    let outside = Vec2::new(q.x.max(0.0), q.y.max(0.0)).length() + q.x.max(q.y).min(0.0) - 0.003;
    let glass = 1.0 - curves::smoothstep(-0.001, 0.001, outside);
    let frame = 1.0 - curves::smoothstep(0.001, 0.003, outside.abs());
    let lit = hash(slot as i32, (uv.x > 0.0) as i32, 61);
    let light = if lit > 0.25 { 0.5 + 0.7 * lit } else { 0.04 };
    // De lejos la hilera se ve como una franja tenue, sin parpadeo.
    let sharp = detail(0.014, lod);
    let coverage = 0.3;
    s.albedo = s.albedo.lerp(Vec3::splat(0.05), glass * sharp).lerp(Vec3::splat(0.18), frame * 0.8 * sharp);
    s.reflectivity *= 1.0 - frame * sharp;
    s.emission = glass * light * sharp + coverage * 0.7 * (1.0 - sharp);
    s.height -= frame * 0.5 * sharp;
    s
}

fn gold_foil(mut s: TextureSample, noise: &NoiseTable, uv: Vec2, lod: f32) -> TextureSample {
    // Pliegues: ruido "ridged" (1 - |n|) en dos escalas. Las crestas finas son
    // las que rompen el reflejo del disco en destellos.
    let q = Vec3::new(uv.x * 42.0, uv.y * 42.0, 3.7);
    let w = noise.value_noise_3d(q * 0.35 + Vec3::splat(9.1));
    let a = 1.0 - noise.value_noise_3d(q + Vec3::splat(w * 1.6)).abs();
    let b = 1.0 - noise.value_noise_3d(q * 2.3 + Vec3::splat(4.2)).abs();
    let coarse = detail(0.024, lod);
    let fine = detail(0.01, lod);
    let crinkle = (a * a * a * 0.7 * coarse + b * b * b * 0.3 * fine) + 0.25 * (1.0 - coarse);
    // Cinta de union entre laminas.
    let tape = (1.0 - curves::smoothstep(0.0, 0.01, ((uv.x * 8.0).fract() - 0.5).abs() - 0.015))
        * detail(0.004, lod);
    let shade = Vec3::new(0.72, 0.52, 0.2).lerp(Vec3::new(1.12, 0.95, 0.6), crinkle);
    s.albedo = (s.albedo * shade).lerp(Vec3::new(0.55, 0.5, 0.42), tape * 0.85);
    s.roughness = 0.14 + 0.2 * (1.0 - crinkle) + 0.15 * (1.0 - fine);
    s.reflectivity = (0.85 + 0.25 * crinkle) * (1.0 - 0.6 * tape);
    s.height = crinkle * 1.4 * coarse - tape * 0.3;
    s
}

fn solar_cells(mut s: TextureSample, uv: Vec2, lod: f32) -> TextureSample {
    let p = Vec2::new(uv.x * 40.0, uv.y * 24.0);
    let cell = p.floor();
    let f = p - cell;
    let gap = f.x.min(1.0 - f.x).min(f.y).min(1.0 - f.y);
    let sharp = detail(0.0012, lod);
    let between = (1.0 - curves::smoothstep(0.03, 0.06, gap)) * sharp;
    // Tres barras colectoras plateadas por celda.
    let bus = (1.0 - curves::smoothstep(0.015, 0.03, ((f.x * 3.0).fract() - 0.5).abs()))
        * detail(0.0008, lod);
    let id = hash(cell.x as i32, cell.y as i32, 29);
    let cell_tint = Vec3::new(0.85 + 0.3 * id, 0.95, 1.0 + 0.25 * (1.0 - id));
    let average = 0.12 * (1.0 - sharp);
    let silicon = s.albedo * cell_tint.lerp(Vec3::ONE, 1.0 - detail(0.025, lod));
    s.albedo = silicon
        .lerp(Vec3::splat(0.55), bus * 0.8)
        .lerp(Vec3::new(0.78, 0.8, 0.82), between + average);
    s.roughness = 0.06 + 0.3 * between;
    s.reflectivity = 1.0 + 1.5 * bus - 0.7 * between;
    s.height = -0.6 * between + 0.25 * bus;
    s
}

fn dome_glass(mut s: TextureSample, noise: &NoiseTable, uv: Vec2, lod: f32) -> TextureSample {
    // `uv` en radianes: azimut y elevacion sobre la base de la cupula.
    // Distancia al nervio mas cercano, en fraccion de vuelta: cero en cada
    // multiplo de 1/8.
    let meridian = ((uv.x * 8.0 / std::f32::consts::TAU + 0.5).fract() - 0.5).abs();
    let parallel = [0.42_f32, 0.9]
        .iter()
        .map(|&e| (uv.y - e).abs())
        .fold(f32::INFINITY, f32::min);
    let base_ring = uv.y;
    let rib = 1.0
        - curves::smoothstep(0.012, 0.02, meridian * 0.8)
            .min(curves::smoothstep(0.016, 0.026, parallel))
            .min(curves::smoothstep(0.03, 0.05, base_ring));
    let rib = rib * detail(0.02, lod) + 0.12 * (1.0 - detail(0.02, lod));
    let smudge = curves::smoothstep(0.5, 0.85, noise.fbm(Vec3::new(uv.x * 2.0, uv.y * 6.0, 2.2), Vec3::ZERO, 4));
    s.transparency = (1.0 - rib) * (1.0 - 0.12 * smudge);
    s.albedo = s.albedo.lerp(Vec3::new(0.22, 0.22, 0.24), rib);
    // El nervio es metal: deja de comportarse como vidrio.
    s.metalness = rib;
    s.roughness = 0.02 + 0.3 * rib + 0.1 * smudge;
    s.reflectivity = 1.0 + 14.0 * rib;
    s.specular = 1.0 - 0.3 * smudge;
    s.height = rib * 0.6;
    s
}

fn heat_tiles(mut s: TextureSample, uv: Vec2, region: Region, lod: f32) -> TextureSample {
    let p = uv * 40.0;
    let cell = p.floor();
    let f = p - cell;
    let gap = f.x.min(1.0 - f.x).min(f.y).min(1.0 - f.y);
    let tiles = detail(0.025, lod);
    let grout = (1.0 - curves::smoothstep(0.02, 0.06, gap)) * detail(0.002, lod);
    let id = hash(cell.x as i32, cell.y as i32, 41);
    // Losetas negras con reemplazos mas claros, como en un transbordador.
    let shade = if id > 0.94 { 3.2 } else { 0.8 + 0.4 * id };
    let shade = 1.0 + (shade - 1.0) * tiles;
    s.albedo = s.albedo * shade * (1.0 - 0.6 * grout) + Vec3::splat(0.02 * grout);
    s.roughness = 0.7 + 0.2 * grout;
    s.height = -grout + 0.08 * id * tiles;
    if region == Region::Instruments {
        // Tablero bajo la cupula: luces de estado en una rejilla, algunas
        // apagadas. Se ven a traves del vidrio, desplazadas por la refraccion.
        let q = uv * 26.0;
        let c = q.floor();
        let d = q - c - Vec2::new(0.5, 0.5);
        let on = hash(c.x as i32, c.y as i32, 77);
        let dot = 1.0 - curves::smoothstep(0.06, 0.13, d.length());
        // Consola de paneles con unas pocas luces encendidas.
        let panel = ((uv.x * 9.0).fract() - 0.5).abs().max(((uv.y * 9.0).fract() - 0.5).abs());
        let frame = curves::smoothstep(0.44, 0.48, panel);
        s.albedo = s.albedo.lerp(Vec3::new(0.2, 0.21, 0.23), 0.7) * (1.0 - 0.5 * frame);
        let sharp = detail(0.008, lod);
        let lights = if on > 0.8 { dot * (0.5 + 3.0 * (on - 0.8)) } else { 0.0 };
        s.emission = lights * sharp + 0.04 * (1.0 - sharp);
    }
    s
}

fn burnt_metal(mut s: TextureSample, noise: &NoiseTable, uv: Vec2, region: Region) -> TextureSample {
    // `uv.y` va de la garganta (0) a la boca (1) de la tobera. El metal
    // calentado se oxida en bandas: paja, bronce, purpura y azul.
    let t = uv.y.clamp(0.0, 1.0);
    let streak = noise.fbm(Vec3::new(uv.x * 9.0, t * 1.5, 5.5), Vec3::ZERO, 4);
    let heat = (t + 0.15 * (streak - 0.5)).clamp(0.0, 1.0);
    let bands = [
        Vec3::new(0.95, 0.8, 0.5),
        Vec3::new(0.85, 0.55, 0.3),
        Vec3::new(0.5, 0.32, 0.6),
        Vec3::new(0.32, 0.45, 0.85),
        Vec3::new(0.55, 0.55, 0.6),
    ];
    let x = heat * (bands.len() - 1) as f32;
    let i = (x.floor() as usize).min(bands.len() - 2);
    let tint = bands[i].lerp(bands[i + 1], x - i as f32);
    s.albedo = (s.albedo * tint * (0.8 + 0.4 * streak) * 1.6).min(Vec3::ONE);
    s.roughness = 0.25 + 0.25 * streak;
    s.height = 0.4 * streak + 0.3 * (t * 18.0).sin().abs();
    if region == Region::Exhaust {
        // Plasma del escape: nucleo azul blanco que se apaga hacia el borde.
        let r = uv.x.clamp(0.0, 1.0);
        s.emission = (1.0 - curves::smoothstep(0.15, 0.95, r)) * (0.85 + 0.3 * streak);
        s.albedo = Vec3::splat(0.05);
        s.reflectivity = 0.0;
    }
    s
}

/// Hash de una celda 2D a `[0, 1)`.
#[inline]
fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8DA6_B343)
        ^ (y as u32).wrapping_mul(0xD816_3841)
        ^ seed.wrapping_mul(0xCB1A_B31F);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h >> 8) as f32 / 16_777_216.0
}

/// Reflexion especular de `d` respecto de la normal unitaria `n`.
#[inline]
pub fn reflect(d: Vec3, n: Vec3) -> Vec3 {
    d - n * (2.0 * d.dot(n))
}

/// Refraccion de Snell con `eta = n1 / n2`. `None` es reflexion total interna.
/// `d` y `n` son unitarios, con `n` del lado de donde viene el rayo.
#[inline]
pub fn refract(d: Vec3, n: Vec3, eta: f32) -> Option<Vec3> {
    let cos_i = -d.dot(n);
    let sin2_t = eta * eta * (1.0 - cos_i * cos_i);
    if sin2_t > 1.0 {
        return None;
    }
    let cos_t = (1.0 - sin2_t).sqrt();
    Some(d * eta + n * (eta * cos_i - cos_t))
}

/// Aproximacion de Schlick a la reflectancia de Fresnel.
#[inline]
pub fn fresnel(cos_theta: f32, f0: f32) -> f32 {
    let m = (1.0 - cos_theta.clamp(0.0, 1.0)).powi(5);
    f0 + (1.0 - f0) * m
}

/// Distribucion de microfacetas GGX (Trowbridge-Reitz) por el sombreado de
/// Smith-Schlick, dividido por `4 (n.v)`: multiplicado por la irradiancia
/// perpendicular de una luz y por Fresnel da la radiancia del brillo.
#[inline]
pub fn ggx(n: Vec3, v: Vec3, l: Vec3, roughness: f32) -> f32 {
    let h = (v + l).normalize_or_zero();
    let n_dot_h = n.dot(h).max(0.0);
    let n_dot_v = n.dot(v).max(1.0e-4);
    let n_dot_l = n.dot(l).max(0.0);
    let alpha = (roughness * roughness).max(1.0e-4);
    let a2 = alpha * alpha;
    let denom = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    let distribution = a2 / (PI * denom * denom);
    let k = (roughness + 1.0) * (roughness + 1.0) / 8.0;
    let g1 = |x: f32| x / (x * (1.0 - k) + k);
    distribution * g1(n_dot_v) * g1(n_dot_l) / (4.0 * n_dot_v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    #[test]
    fn every_material_has_its_own_texture_and_valid_parameters() {
        for (i, a) in MATERIALS.iter().enumerate() {
            for b in &MATERIALS[i + 1..] {
                assert_ne!(a.texture, b.texture, "{} y {}", a.name, b.name);
            }
            for value in [a.specular, a.roughness, a.transparency, a.reflectivity, a.metalness] {
                assert!((0.0..=1.0).contains(&value), "{}", a.name);
            }
            assert!(a.albedo.min_element() >= 0.0 && a.albedo.max_element() <= 1.0);
            assert!(a.ior >= 1.0);
        }
        assert!(MaterialId::Glass.material().transparency > 0.5);
        assert!(MaterialId::Glass.material().ior > 1.0);
        assert!(MaterialId::Gold.material().reflectivity > 0.3);
    }

    #[test]
    fn textures_vary_up_close_and_settle_far_away() {
        let noise = NoiseTable::new(config::NOISE_SEED);
        let spread = |material: &Material, lod: f32| {
            let values: Vec<f32> = (0..96)
                .map(|i| {
                    let uv = Vec2::new(i as f32 * 0.0173, (i * 7 % 13) as f32 * 0.0211);
                    let s = material.sample(&noise, uv, Region::Plain, lod);
                    s.albedo.luminance() + s.height + s.transparency
                })
                .collect();
            let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
            let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            max - min
        };
        for material in &MATERIALS {
            let near = spread(material, 1.0e-4);
            assert!(near > 0.05, "{} es uniforme de cerca", material.name);
        }
        // Con pixeles mas grandes que los paneles, el casco deja de titilar.
        let hull = MaterialId::Hull.material();
        assert!(spread(hull, 0.5) < 0.5 * spread(hull, 1.0e-4));
    }

    #[test]
    fn snell_fresnel_and_reflection_follow_the_optics() {
        let n = Vec3::Y;
        let d = Vec3::new(0.6, -0.8, 0.0);
        let r = reflect(d, n);
        assert!((r - Vec3::new(0.6, 0.8, 0.0)).length() < 1e-6);
        // n1 sin(t1) = n2 sin(t2), aire a vidrio.
        let t = refract(d, n, 1.0 / 1.5).unwrap();
        assert!((t.length() - 1.0).abs() < 1e-5);
        assert!((0.6 - 1.5 * t.x).abs() < 1e-5 && t.y < 0.0);
        // Del vidrio al aire con angulo mayor al critico: reflexion total.
        let grazing = Vec3::new(0.8, 0.6, 0.0);
        assert!(refract(grazing, -n, 1.5).is_none());
        let f0 = ((1.5_f32 - 1.0) / (1.5 + 1.0)).powi(2);
        assert!((fresnel(1.0, f0) - f0).abs() < 1e-6);
        assert!((fresnel(0.0, f0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ggx_distribution_is_normalized_over_the_hemisphere() {
        // La integral de D(h) (n.h) sobre el hemisferio vale 1 para cualquier
        // rugosidad: los brillos no inventan energia.
        for roughness in [0.25_f32, 0.5, 0.8] {
            let alpha = roughness * roughness;
            let a2 = alpha * alpha;
            let steps = 4000;
            let mut integral = 0.0;
            for i in 0..steps {
                let theta = (i as f32 + 0.5) / steps as f32 * PI / 2.0;
                let c = theta.cos();
                let denom = c * c * (a2 - 1.0) + 1.0;
                let d = a2 / (PI * denom * denom);
                integral += d * c * theta.sin() * (PI / 2.0 / steps as f32) * 2.0 * PI;
            }
            assert!((integral - 1.0).abs() < 0.02, "rugosidad {roughness}: {integral}");
        }
        let n = Vec3::Y;
        let v = Vec3::new(0.0, 1.0, 0.0);
        assert!(ggx(n, v, v, 0.3) > ggx(n, v, Vec3::new(0.8, 0.6, 0.0), 0.3));
    }
}
