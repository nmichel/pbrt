use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, Intersection, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::geom::vector3::Vector3f;

use super::Shape;

pub struct Plane {}

impl Plane {
    pub fn new() -> Self {
        Self {}
    }
}

impl Shape for Plane {}

impl Intersectable for Plane {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        let mut res = IntersectionResult::new();
        let inv_dir = 1.0 / ray.direction.y; // Will be set to +INF if ray.direction.y is 0
        let d = (ray.origin.y * -1.0) / inv_dir;
        if d < near || d > far {
            return res;
        }

        let mut p = ray.origin + ray.direction * d;
        p.y = 0.0;

        res.push(Intersection {
            p,
            d,
            n: vector3::Vector3f::new(0.0, 1.0, 0.0),
            wo: &ray.direction * -1.0,
            u: p.x,
            v: p.z,
            dpdu: vector3::Vector3::new(1.0, 0.0, 0.0),
            dpdv: vector3::Vector3::new(0.0, 0.0, 1.0),
        });

        res
    }

    fn contain_point(&self, point: &Vector3f) -> bool {
        point.y < 0.0
    }
}

impl AABound for Plane {
    /// Bounds the **solid**, which is the half-space below the surface, not the surface itself.
    ///
    /// `contain_point` answers `point.y < 0.0`: this shape *is* the half-space y ≤ 0, and
    /// `intersect` returns where a ray crosses its boundary — the usual convention, where a shape is
    /// a volume and its intersection routine reports its skin. A bound must contain the object it
    /// bounds, so it has to reach down to −∞ in y. A slab of zero thickness at y = 0 would bound the
    /// skin while excluding everything under it, and a constructive intersection reading that bound
    /// would clip away real geometry: `Intersection(sphere, plane)` is the lower half of the sphere,
    /// and a bound flat at y = 0 keeps none of it.
    ///
    /// Infinite in x and z, and stated as `±f64::INFINITY` rather than `±f64::MAX`. The difference is
    /// not cosmetic. `f64::MAX` is a *finite* number standing in for infinity: the box it describes
    /// reports an infinite area anyway, since `bmax - bmin` overflows, but it does so by accident and
    /// in a way no predicate can distinguish from a merely enormous box. Stated as an infinity,
    /// `AABoundingBox::is_bounded` can tell — and `Scene` uses it to keep this primitive out of the
    /// accelerator, which is where an unbounded primitive belongs (see
    /// `docs/heuristique_aire_surface.md` §5).
    fn get_bounding_box(&self) -> AABoundingBox {
        let bmin = Vector3f::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        let bmax = Vector3f::new(f64::INFINITY, 0.0, f64::INFINITY);
        AABoundingBox::new(&bmin, &bmax)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bound must contain the solid, and the solid is everything under y = 0.
    ///
    /// A bound stopping at y = 0 would describe the skin instead, and any constructive intersection
    /// reading it would clip the volume away: a bound is the one promise the accelerator and the CSG
    /// operators both rely on, and it is a promise about the object, not about its surface.
    #[test]
    fn test_the_bound_contains_the_solid_and_not_just_the_surface() {
        let plane = Plane::new();
        let bbox = plane.get_bounding_box();

        // A point well inside the half-space, per `contain_point`.
        let deep_inside = Vector3f::new(5.0, -3.0, -7.0);
        assert!(plane.contain_point(&deep_inside));
        assert!(
            bbox.bmin.y <= deep_inside.y && deep_inside.y <= bbox.bmax.y,
            "the bound must reach below the surface, got y ∈ [{}, {}]",
            bbox.bmin.y,
            bbox.bmax.y
        );

        // And it stops at the surface: nothing above y = 0 belongs to the solid.
        assert_eq!(bbox.bmax.y, 0.0);
        assert!(!plane.contain_point(&Vector3f::new(0.0, 1.0, 0.0)));

        assert!(!bbox.is_bounded(), "infinite in x and z, so it stays out of the accelerator");
    }
}
