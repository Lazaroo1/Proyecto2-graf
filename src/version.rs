//! Versiones visuales del proyecto. `V` las recorre en este orden.
//!
//! La original y la variante muestran solo el agujero negro. La version
//! Endurance agrega la nave: superficies con materiales, reflexion, refraccion
//! y un skybox, todas trazadas sobre las mismas geodesicas.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    Original,
    Variante,
    Endurance,
}

impl Version {
    /// Version de arranque segun los argumentos de la linea de comandos.
    pub fn from_args(args: &[String]) -> Self {
        if args.iter().any(|arg| arg == "--original") {
            Self::Original
        } else if args.iter().any(|arg| arg == "--endurance") {
            Self::Endurance
        } else {
            Self::Variante
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Original => Self::Variante,
            Self::Variante => Self::Endurance,
            Self::Endurance => Self::Original,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Variante => "variante",
            Self::Endurance => "Endurance",
        }
    }

    /// Gas filamentoso con envoltura gris: la variante y la Endurance lo comparten.
    pub fn enhanced(self) -> bool {
        self != Self::Original
    }

    /// Identificador estable para la clave de cache.
    pub fn index(self) -> u32 {
        self as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v_cycles_through_the_three_versions() {
        let start = Version::Original;
        assert_eq!(start.next(), Version::Variante);
        assert_eq!(start.next().next(), Version::Endurance);
        assert_eq!(start.next().next().next(), start);
        let args = |a: &str| vec![a.to_string()];
        assert_eq!(Version::from_args(&args("--original")), Version::Original);
        assert_eq!(Version::from_args(&args("--endurance")), Version::Endurance);
        assert_eq!(Version::from_args(&[]), Version::Variante);
        assert!(!Version::Original.enhanced() && Version::Endurance.enhanced());
    }
}
