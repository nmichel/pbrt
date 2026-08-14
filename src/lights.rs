use super::geom::intersectable::Intersection;
use super::geom::ray::Ray;
use super::geom::vector3::Vector3f;
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
    pub fn between(from: &Vector3f, to: &Vector3f) -> Self {
        // `spawn_from_through` normalises the direction, so the parametric distance along the ray
        // and the euclidean distance to the light are the same number.
        Self {
            ray: Ray::spawn_from_through(from, to),
            distance: (to - from).length(),
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
}

/// This struct captures the result of sampling a light source at a given shading point.
pub struct LightLiSample {
    /// The spectral radiance received from the sampled point on the light source.
    pub spectrum: Spectrum,

    /// The direction from the shading point to the sampled point on the light source.
    pub wi: Vector3f,

    /// The probability density function value associated with sampling `wi`.
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

    fn sample_li(&self, _intersection: &Intersection) -> Option<(LightLiSample, VisibilityTester)>;
}

mod background_infinite_light;
mod point_light;
mod uniform_infinite_light;

pub use self::background_infinite_light::BackgroundInfiniteLight;
pub use self::point_light::PointLight;
pub use self::uniform_infinite_light::UniformInfiniteLight;
