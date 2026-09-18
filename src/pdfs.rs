use crate::geom::vector2::Vector2f;
use crate::geom::vector3::Vector3f;

/// A probability density over directions, and the map that draws from it.
///
/// Reference: PBR Book, 3ed, §13.6 — *2D Sampling with Multidimensional Transformations*.
/// <https://www.pbr-book.org/3ed-2018/Monte_Carlo_Integration/2D_Sampling_with_Multidimensional_Transformations>
///
/// **Frame**: directions are unit vectors of the shading frame — see
/// [`ShadingFrame`](crate::geom::shading_frame::ShadingFrame), which states the convention and holds
/// the change of basis. `value` takes a direction of that frame and returns a density **per unit
/// solid angle**.
///
/// # The two methods, and the contract between them
///
/// `generate` draws a direction; `value` gives the density at a direction. A Monte-Carlo estimator
/// divides by that density, so the two must describe the *same* distribution — a `value` that does
/// not integrate to 1 over the directions `generate` can produce biases every estimator built on
/// it. That is what the energy-conservation tests of the implementations check, and why a new pdf
/// must arrive with one.
///
/// # Why `generate` takes its sample rather than drawing it
///
/// A pdf **is** a map from the unit square to directions — the inverse of its cumulative
/// distribution. Taking `u` as an argument says so in the signature, and buys three things a pdf
/// that reached for a generator itself could not have:
///
/// - `pdfs` depends on no source of randomness, so it cannot be the reason a render stops being
///   reproducible;
/// - a test can feed a chosen `u`, or a regular grid, and check the map itself rather than a
///   sample of its output;
/// - where the numbers come from is decided by the caller. A sampler that spreads its samples
///   deliberately reaches every pdf without any of them changing.
///
/// All three implementations consume exactly two numbers, which is the dimension of a direction:
/// one for the azimuth φ, one for the polar angle through its own inverse cdf.
pub trait Pdf {
    fn value(&self, _direction: &Vector3f) -> f64;

    /// The direction that `u ∈ [0,1)²` maps to under this density.
    fn generate(&self, u: &Vector2f) -> Vector3f;
}

pub mod cosine;
pub mod hemisphere;
pub mod sphere;
