use super::intersectable::{Intersectable, Intersection};
use super::ray::Ray;
use super::vector3;
use super::vector3::Vector3f;

pub struct Sphere {
    o: Vector3f,
    r: f64
}

impl Sphere {
    pub fn new(o: Vector3f, r: f64) -> Sphere {
        Sphere { o: o, r: r }
    }

    pub fn center(&self) -> &Vector3f {
        &self.o
    }

    pub fn radius(&self) -> f64 {
        self.r
    }
}

impl Intersectable for Sphere {
    fn intersect(&self, ray: &Ray) -> Option<Intersection> {
        let t0: f64;
        let t1: f64;
        let L = &self.o - &ray.origin; 
        let tca = vector3::dot(&L, &ray.direction);
        if tca < 0.0 {
            return None;
        }

        let radius2 = self.r * self.r;
        let d2 = vector3::dot(&L, &L) - tca * tca;
        if d2 > radius2 {
            return None;
        }

        let thc = (radius2 - d2).sqrt();
        t0 = tca - thc;
        t1 = tca + thc;

        let t = f64::min(t0, t1);

        let hit = &ray.origin + &(&ray.direction * t);
        let mut norm = &hit - &self.o;
        norm.normalize();

        Some(Intersection {
            p: hit,
            d: t,
            n: norm,
            wo: &ray.direction * -1.0
        })
    }
}
