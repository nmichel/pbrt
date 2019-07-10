use super::intersectable::{Intersectable, Intersection};
use super::ray::Ray;
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
    fn intersect(&self, ray: &Ray) -> Vec<Intersection> {
        vec![]
    }
}
