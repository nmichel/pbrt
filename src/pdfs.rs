use crate::geom::vector2::Vector2f;
use crate::geom::vector3::Vector3f;

/// A probability density over directions, and the map that draws from it.
///
/// Reference: PBR Book, 4ed, §A.5 — *Sampling Multidimensional Functions*.
/// <https://pbr-book.org/4ed/Sampling_Algorithms/Sampling_Multidimensional_Functions>
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

/// How a sampled direction is distributed, and so what an estimator may divide by.
///
/// Reference: PBR Book, 4ed, §9.1.2 — *BSDF Representation*, whose `BSDFSample` carries this
/// distinction as a flag, `IsSpecular()`; and §12.1 — *Light Interface*, whose `LightType` tells a
/// δ source from one with a density.
/// <https://pbr-book.org/4ed/Reflection_Models/BSDF_Representation>
/// <https://pbr-book.org/4ed/Light_Sources/Light_Interface>
///
/// # Why the sample says it
///
/// A Monte-Carlo estimator divides by the density of what it drew. That density exists only if the
/// distribution has one: a perfect mirror reflects into a single direction, a point light is seen
/// along a single direction, and the distribution of either is a δ — which has no value per unit
/// solid angle that one could divide by.
///
/// Whether a draw came from one kind or the other is a property of **the draw**, not of what drew
/// it: a material with a mirror lobe and a diffuse lobe, picking one at random, produces both. So
/// the answer travels with each sample — [`ScatterInfo`](crate::materials::ScatterInfo) for a bsdf,
/// [`LightLiSample`](crate::lights::LightLiSample) for a source — and the integrator reads it there.
///
/// # Why an enum, against CLAUDE.md §2
///
/// The project adds variants through trait implementations, not through a `match` in the caller.
/// This is not a variant of a concept: a measure is either absolutely continuous or singular, there
/// is no third case to add later, and an integrator has to treat the two differently — which is
/// exactly what a closed sum type says.
///
/// # Two simpler shapes, and why neither is taken
///
/// **Handing over the combined weight** — `f · |cos θᵢ| / p` for a bsdf — would remove the
/// distinction altogether: it is all a path tracer multiplies `β` by, and for a δ it is simply ρ.
/// It is not taken because multiple importance sampling needs the density **on its own**, to weigh
/// a bsdf sample against a light sample, and a product cannot be split back into its factors.
///
/// **Keeping one number, and putting the fiction in it** — a "density" of 1 for a δ, with a mirror's
/// attenuation divided by the cosine its caller will apply, which is pbrt-v3's convention — makes
/// the integrator's general formula come out right. The number is then not a density, and the
/// first consumer to take it at its word, multiple importance sampling, computes a meaningless
/// weight. A wrong attenuation shows on an image; a wrong density corrupts whatever is computed
/// from it next.
pub enum DirectionDensity {
    /// A density per unit solid angle, in world space, at the point the direction is drawn from.
    SolidAngle(f64),

    /// A δ distribution: the direction is the only one that could have been produced, and there is
    /// no density to divide by.
    Dirac,
}

pub mod cosine;
pub mod hemisphere;
pub mod sphere;
