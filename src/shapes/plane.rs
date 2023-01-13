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
    fn get_bounding_box(&self) -> AABoundingBox {
        let mut bmin = Vector3f::min();
        bmin.y = -0.01;
        let mut bmax = Vector3f::max();
        bmax.y = 0.01;
        AABoundingBox::new(&bmin, &bmax)
    }
}
