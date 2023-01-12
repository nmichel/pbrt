use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, Intersection, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::geom::vector3::Vector3f;

pub struct Rectangle {
    half_width: f64,
    half_height: f64,
}

impl Rectangle {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            half_width: width / 2.0,
            half_height: height / 2.0,
        }
    }
}

impl Intersectable for Rectangle {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        let mut res = IntersectionResult::new();

        if ray.direction.y == 0.0 {
            res
        }
        else {
            let d = (ray.origin.y * -1.0) / ray.direction.y;
            if d < near || d > far {
                return res;
            }

            let mut p = ray.origin + ray.direction * d;
            p.y = 0.0;

            if p.x.abs() > self.half_width || p.z.abs() > self.half_height {
                return res;
            }

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
    }

    fn contain_point(&self, point: &Vector3f) -> bool {
        point.y < 0.0 && point.x.abs() <= self.half_width && point.z.abs() <= self.half_height
    }
}

impl AABound for Rectangle {
    fn get_bounding_box(&self) -> AABoundingBox {
        let bmin = Vector3f::new(-self.half_width, -1.0, -self.half_height);
        let bmax = Vector3f::new(self.half_width, 1.0, self.half_height);
        AABoundingBox::new(&bmin, &bmax)
    }
}
