use crate::geom::intersectable::{Intersectable, Intersection};
use crate::geom::ray::Ray;
use crate::geom::vector3;
use num_traits::clamp;

const PI: f64 = 3.14159265358979323846;

pub struct Sphere {
    r: f64
}

impl Sphere {
    pub fn new(r: f64) -> Sphere {
        Sphere { r }
    }

    pub fn radius(&self) -> f64 {
        self.r
    }
}

impl Intersectable for Sphere {
    fn intersect(&self, ray: &Ray) -> Option<Intersection> {
        let l = &ray.origin * -1.0; 
        let tca = vector3::dot(&l, &ray.direction);
        if tca < 0.0 {
            return None;
        }

        let r2 = self.r * self.r;
        let d2 = vector3::dot(&l, &l) - tca * tca;
        if d2 > r2 {
            return None;
        }

        let thc = (r2 - d2).sqrt();
        let t0: f64 = tca - thc;
        let t1: f64 = tca + thc;
        let t = f64::min(t0, t1);

        let mut hit = &ray.origin + &(&ray.direction * t);
        let mut norm = hit;
        norm.normalize();

        if hit.x == 0.0 && hit.y == 0.0 {
            hit.x = 1.0e-5 * self.r;
        }

        let mut phi = hit.y.atan2(hit.x);
        if phi < 0.0 {
            phi += 2.0 * PI;
        }
        let u = phi / (2.0 * PI);

        let theta = clamp(hit.z / self.r, -1.0, 1.0).acos();
        let v = theta / PI;

        Some(Intersection {
            p: hit,
            d: t,
            n: norm,
            wo: &ray.direction * -1.0,
            u,
            v
        })
    }
}
