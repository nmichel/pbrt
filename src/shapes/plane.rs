use crate::geom::intersectable::{Intersectable, Intersection};
use crate::geom::ray::Ray;
use crate::geom::vector3;

pub struct Plane {}

impl Plane {
    pub fn new() -> Self {
        Self {}
    }
}

impl Intersectable for Plane {
    fn intersect(&self, ray: &Ray) -> Option<Intersection> {
        if ray.direction.y == 0.0 {
            None
        }
        else {
            let d = (ray.origin.y * -1.0) / ray.direction.y;
            if d <= 0.0 {
                return None;
            }

            let mut p = ray.origin + ray.direction * d;
            p.y = 0.00001;

            Some(Intersection {
                p,
                d,
                n: vector3::Vector3f::new(0.0, 1.0, 0.0),
                wo: &ray.direction * -1.0,
                u: p.x,
                v: p.z,
                dpdu: vector3::Vector3::new(1.0, 0.0, 0.0),
                dpdv: vector3::Vector3::new(0.0, 0.0, 1.0)
            })
        }
    }
}