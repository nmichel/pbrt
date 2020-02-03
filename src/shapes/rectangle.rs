use crate::geom::intersectable::{Intersectable, Intersection};
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::geom::vector3::Vector3f;

pub struct Rectangle {
    half_width: f64,
    half_height: f64
}

impl Rectangle {
    pub fn new(width: f64, height: f64) -> Self {
        Self { half_width: width/2.0, half_height: height/2.0 }
    }
}

impl Intersectable for Rectangle {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Intersection> {
        if ray.direction.y == 0.0 {
            None
        }
        else {
            let d = (ray.origin.y * -1.0) / ray.direction.y;
            if d < near || d > far {
                return None;
            }

            let mut p = ray.origin + ray.direction * d;
            p.y = 0.0;

            if p.x.abs() > self.half_width || p.z.abs() > self.half_height {
                return None;
            }

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

    fn contain_point(&self, point: &Vector3f) -> bool {
        point.y < 0.0 && point.x.abs() <= self.half_width && point.z.abs() <= self.half_height
    }
}
