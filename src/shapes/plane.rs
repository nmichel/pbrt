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
    /// The plane is infinite in x and z, and says so.
    ///
    /// It used to say `±f64::MAX`, which is a *finite* number standing in for infinity, and the
    /// difference is not cosmetic: `bmax - bmin` then overflows to `+inf` and the box reports an
    /// infinite area anyway, but by accident, in a way no predicate can distinguish from a merely
    /// enormous box. Written honestly, `AABoundingBox::is_bounded` can tell — and `Scene` uses it
    /// to keep this primitive out of the accelerator entirely, which is where an unbounded
    /// primitive belongs (see `docs/heuristique_aire_surface.md` §5).
    ///
    /// Zero thickness in y is exact and correct: the plane really is flat. `hit` handles a
    /// zero-extent slab, and `half_area` reports the bound faithfully rather than inflating it.
    fn get_bounding_box(&self) -> AABoundingBox {
        let bmin = Vector3f::new(f64::NEG_INFINITY, 0.0, f64::NEG_INFINITY);
        let bmax = Vector3f::new(f64::INFINITY, 0.0, f64::INFINITY);
        AABoundingBox::new(&bmin, &bmax)
    }
}
