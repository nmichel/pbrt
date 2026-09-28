//! The sources a scene is lit by, and what each of them hands an integrator.
//!
//! Reference: PBR Book, 4ed, chapter 12 — *Light Sources*.
//! <https://pbr-book.org/4ed/Light_Sources>
//!
//! The derivations — the radiometric quantities, the area-to-solid-angle jacobian, and the two
//! unrelated reasons an inverse square shows up — are in `docs/sources_de_lumiere.md`. This file
//! states only the contract the three families have in common.

use super::geom::intersectable::Intersection;
use super::geom::ray::Ray;
use super::geom::vector3::Vector3f;
use super::samplers::Sampler;
use super::scene::Scene;
use super::spectrum::Spectrum;

/// Where a shadow ray starts along its own direction.
///
/// A shadow ray leaves the very surface being shaded, so at t = 0 it sits *on* that surface, and
/// the slab and triangle tests — deliberately conservative, and working in floating point — may
/// well report it as hitting the surface it started from. Every such ray would then be occluded
/// by its own origin, and every directly lit point would come out black. Stepping a little way
/// along the ray avoids it, at the price of missing a genuine occluder within 10⁻⁵ of the shaded
/// point. That is the trade renderers make under the name *shadow acne*.
///
/// **A departure worth naming**: this is an *absolute* distance, so it is scene-scale dependent —
/// right for scenes spanning a few units, and wrong for a scene measured in kilometres, where
/// 10⁻⁵ falls below the spacing of the floats involved and stops separating anything. The
/// principled form is a bound relative to the magnitudes at play, as `AABoundingBox::hit` already
/// uses for its slabs; see `docs/arithmetique_flottante.md` §1. Not done here.
const SHADOW_RAY_EPSILON: f64 = 0.00001;

/// How much of the segment to the light is left untested, as a fraction of its length.
///
/// The far end of a shadow ray is the mirror image of the near end, and it bites hardest on an
/// area light: the point sampled on the source **is a point of a surface in the scene**, so the
/// segment ends exactly on that surface, and `intersect_p` — which accepts a hit at `t == far` —
/// reports the source as standing in its own way. Measured on the panel of
/// `test_files/cornell_box_exact.stage`, from a floor point that sees it straight up, three
/// shadow rays in four came back occluded by the panel itself.
///
/// It cannot be repaired at the other end. Stepping off the *shaded* surface, as
/// [`SHADOW_RAY_EPSILON`] does, says nothing about where the segment stops; and no tightening of
/// the comparison helps, rounding putting the computed distance on either side of the exact one.
/// The segment has to stop short.
///
/// **This one is relative**, unlike its counterpart above, and deliberately: what has to be
/// cleared is a rounding error on the distance itself, which grows with that distance. The price
/// is a genuine occluder within the last hundredth of a percent of the way going unseen — a
/// lampshade pressed against a lamp.
const SHADOW_RAY_END_EPSILON: f64 = 0.0001;

/// The occlusion query that comes with a light sample: is anything standing between the shaded
/// point and the light?
///
/// It carries a ray **and how far along it the light sits**, because that distance is the bound of
/// the search — not something to check afterwards. Everything beyond the light is irrelevant to
/// the question and must not be looked at.
pub struct VisibilityTester {
    ray: Ray,

    /// Parametric distance at which the light sits. `f64::MAX` for a light at infinity, which is
    /// not a special case but the honest value: there is no point beyond which an occluder would
    /// stop mattering.
    distance: f64,
}

impl VisibilityTester {
    /// Visibility of a light sitting at `to`, seen from `from`.
    ///
    /// The segment stops just short of `to`, by [`SHADOW_RAY_END_EPSILON`]: a point drawn on an
    /// area light lies on a surface the accelerator knows, and a segment reaching it exactly finds
    /// that surface in its own way.
    pub fn between(from: &Vector3f, to: &Vector3f) -> Self {
        // `spawn_from_through` normalises the direction, so the parametric distance along the ray
        // and the euclidean distance to the light are the same number.
        Self {
            ray: Ray::spawn_from_through(from, to),
            distance: (to - from).length() * (1.0 - SHADOW_RAY_END_EPSILON),
        }
    }

    /// Visibility of a light infinitely far away, seen from `from` in direction `wi`.
    ///
    /// A light at infinity has no position to aim at, only a direction — which is exactly why the
    /// two-point form cannot express it. Trying to force it produced a segment from the origin to
    /// itself, a ray of zero length whose normalised direction was `NaN`; it passed through the
    /// whole scene without meeting anything, so these lights cast no shadow at all.
    pub fn towards_infinity(from: &Vector3f, wi: &Vector3f) -> Self {
        Self {
            ray: Ray::new(from, wi),
            distance: f64::MAX,
        }
    }

    /// Whether the light is visible from the shaded point.
    ///
    /// Asks `intersect_p`, not `intersect`: *any* occluder settles the question, so there is no
    /// reason to rank candidates by distance, nor to build the shading frame and material of a
    /// surface whose only role is to be in the way.
    pub fn unoccluded(&self, scene: &Scene) -> bool {
        !scene.intersect_p(&self.ray, SHADOW_RAY_EPSILON, self.distance)
    }
}

#[derive(Debug, PartialEq)]
pub enum LightType {
    Point,
    Infinite,

    /// A source with an extent, which a ray can therefore hit. That is what sets it apart from the
    /// other two: a point has zero probability of being hit, and an infinite light is only ever
    /// reached by escaping the scene.
    Area,
}

/// This struct captures the result of sampling a light source at a given shading point.
///
/// # The contract, which is the seam
///
/// An integrator knows nothing of light sources: it weights `spectrum` by the bsdf and a cosine and
/// divides by `pdf`. That estimator is only unbiased if every source agrees on what the three
/// fields mean, and the agreement is a matter of **measure**:
///
/// - `pdf` is a density **per unit solid angle at the shaded point**, for the direction `wi`;
/// - `spectrum` is the radiance arriving from `wi`, and radiance does not fall off with distance.
///
/// The inverse square law therefore belongs to whichever of the two the source's own physics puts
/// it in — the density for a source with an area, the radiance for a point source, which has an
/// intensity and no radiance at all. Writing it in both is the mistake the measure conventions
/// exist to prevent, and it only changes how bright an image is, never its shape.
pub struct LightLiSample {
    /// The spectral radiance received from the sampled point on the light source.
    pub spectrum: Spectrum,

    /// The direction from the shading point to the sampled point on the light source.
    pub wi: Vector3f,

    /// The density of `wi`, per unit solid angle at the shaded point.
    ///
    /// A source described by a delta distribution has no such density: it answers `1.0`, the
    /// neutral element that makes the integrator's division harmless. That value is not a density,
    /// which is why multiple importance sampling will have to tell the two cases apart.
    pub pdf: f64,
}

pub trait Light: Send + Sync {
    /// Returns the type of light
    ///
    /// Depending on the context some light types may or may not be used.
    fn light_type(&self) -> LightType;

    fn le(&self, _ray: &Ray) -> Spectrum {
        Spectrum::new(0.0, 0.0, 0.0)
    }

    /// Samples a point on the light as seen from `intersection`, drawing from `sampler` if the
    /// source has an extent. A point light needs no number at all.
    fn sample_li(&self, _intersection: &Intersection, _sampler: &mut dyn Sampler) -> Option<(LightLiSample, VisibilityTester)>;
}

mod area_light;
mod background_infinite_light;
mod point_light;
mod uniform_infinite_light;

pub use self::area_light::AreaLight;
pub use self::background_infinite_light::BackgroundInfiniteLight;
pub use self::point_light::PointLight;
pub use self::uniform_infinite_light::UniformInfiniteLight;
