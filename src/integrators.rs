use crate::colors;
use crate::geom::ray::Ray;
use crate::samplers::Sampler;
use crate::scene::Scene;
use crate::spectrum::Spectrum;
use std::fmt;
use std::str::FromStr;

pub trait Integrator: Send + Sync {
    /// Radiance arriving along `ray`, estimated by drawing from `sampler`.
    ///
    /// The sampler is the integrator's only source of numbers, and it hands it on to the
    /// materials and lights it queries: a path is therefore a deterministic function of the
    /// stream it is given.
    fn li(&self, ray: &Ray, scene: &Scene, depth: usize, near: f64, far: f64, sampler: &mut dyn Sampler) -> Spectrum;

    fn background_radiance(&self, _ray: &Ray, _scene: &Scene) -> Spectrum {
        colors::BLACK
    }
}

pub mod naive;
pub mod normal;
pub mod path;
// pub mod whitted;

pub use self::naive::NaiveIntegrator;
pub use self::normal::NormalIntegrator;
pub use self::path::PathIntegrator;

/// Which [`Integrator`] a run builds.
///
/// The mapping between a variant and the name the command line uses lives here rather than in
/// the caller that reads the option: adding an integrator is then a change to this module alone.
#[derive(Debug, PartialEq)]
pub enum Type {
    PATH,
    NAIVE,
    NORMAL,
}

impl Type {
    /// The accepted names, phrased for whoever mistyped one.
    pub const ACCEPTED_VALUES: &'static str = "one of path, naive, normal";
}

/// Writes the name the command line uses, so a printed configuration reads back as a command
/// line that reproduces it — `to_string` and [`FromStr`] are inverse, which the doc-test below
/// checks.
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Type::PATH => "path",
            Type::NAIVE => "naive",
            Type::NORMAL => "normal",
        };
        write!(f, "{}", name)
    }
}

/// ```
/// use pbrt::integrators::Type;
/// use std::str::FromStr;
///
/// for name in ["path", "naive", "normal"].iter() {
///     assert_eq!(*name, Type::from_str(name).unwrap().to_string());
/// }
/// assert!(Type::from_str("quantum").is_err());
/// ```
impl FromStr for Type {
    /// Nothing to carry: the message a user reads is built by the caller, which knows the
    /// option name the value was given to. [`Type::ACCEPTED_VALUES`] supplies the rest.
    type Err = ();

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "path" => Ok(Type::PATH),
            "naive" => Ok(Type::NAIVE),
            "normal" => Ok(Type::NORMAL),
            _ => Err(()),
        }
    }
}
