use super::ray::Ray;
use super::vector3::Vector3f;
use crate::spectrum::Spectrum;
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub struct Intersection {
    /// Intersection point
    pub p: Vector3f,

    /// Distance to the `Ray` origin
    pub d: f64,

    /// Normal vector at intersection point
    pub n: Vector3f,

    /// Inverse direction of the `Ray`
    pub wo: Vector3f,

    /// U/V coordinate of intersection point on the surface
    pub u: f64,
    pub v: f64,

    /// Position derivatives at intersection point
    pub dpdu: Vector3f,
    pub dpdv: Vector3f,
}

pub type IntersectionResult = Vec<Intersection>;

pub trait Intersectable: Send + Sync {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult;

    /// Whether `ray` meets this geometry at all within `[near, far]`.
    ///
    /// A different question from `intersect`, and one that deserves different work: *any* hit
    /// answers it, so there is no nearest to look for, no distance to rank, and none of the
    /// shading frame — normal, texture coordinates, ∂p/∂u, ∂p/∂v — that a surface whose only role
    /// is to be in the way will never be asked for. Shadow rays ask this and nothing else.
    ///
    /// The default implementation answers it by way of `intersect`, which is **correct for every
    /// implementation and wasteful for most**: it computes all of the above and then looks only at
    /// whether the list came back empty. It is a default so that adding this method breaks
    /// nothing, not because it is a good way to answer. Override it wherever the waste is worth
    /// removing — which means wherever `intersect` does real work, and not in a shape whose
    /// intersection is a handful of arithmetic.
    fn intersect_p(&self, ray: &Ray, near: f64, far: f64) -> bool {
        !self.intersect(ray, near, far).is_empty()
    }

    /// Whether `point` belongs to the solid this shape encloses.
    ///
    /// The constructive operators of `shapes::csg` are the only consumers: they take the boundary
    /// points `intersect` reports and keep or discard each one according to whether it lies inside
    /// the other members. So this is the *interior* half of a shape's description, `intersect` being
    /// the boundary half — and the two must agree, since nothing enforces it. A `contain_point`
    /// describing a taller solid than `intersect` bounds carves holes through geometry the shape
    /// never reached.
    ///
    /// # Conventions
    ///
    /// **The solid is closed**: a point exactly on the boundary is inside it. The comparisons are
    /// therefore `<=` and not `<`. The alternative is defensible — an open solid — but the choice
    /// has to be the same everywhere, because it is asked precisely about points that lie *on* some
    /// surface, and mixing the two makes "on the boundary" mean inside for one shape and outside for
    /// its neighbour. Closed follows the reasoning `AABoundingBox::hit` already applies to a
    /// tangential hit: at the boundary, keep rather than drop.
    ///
    /// Note that neither choice is forgiving in general. An intersection drops a coincident surface
    /// under the open convention, a union drops it under the closed one — which is the classic
    /// argument for deriving the interior from the ray's own crossings instead of asking a separate
    /// predicate. That is a redesign, not a convention.
    ///
    /// **An open surface encloses nothing**, and answers `false`. A `Rectangle` or a `TriangleMesh`
    /// has no inside to speak of: any answer would be arbitrary, and `false` is the one that keeps
    /// such a shape from claiming a volume a constructive operator would then cut against.
    fn contain_point(&self, point: &Vector3f) -> bool;
}

impl Intersection {
    pub fn le(&self, _wo: &Vector3f) -> Spectrum {
        // match self.primitive.getAreaLight() {
        //     Some(light) => light.l(self, wo),
        //     None => Spectrum::new(0.0, 0.0, 0.0)
        // }
        //
        // Pour l'instant, retourne "rien"
        Spectrum::new(0.0, 0.0, 0.0)
    }
}

impl fmt::Display for Intersection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[position: {}, distance: {}, normal: {}]", self.p, self.d, self.n)
    }
}
