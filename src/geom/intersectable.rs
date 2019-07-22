use super::ray::Ray;
use super::vector3::Vector3f;
use std::fmt;

#[derive(Debug)]
pub struct Intersection {
    pub p: Vector3f,
    pub d: f64,
    pub n: Vector3f
}

pub trait Intersectable {
    fn intersect(&self, ray: &Ray) -> Vec<Intersection>;
}

impl fmt::Display for Intersection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[position: {}, distance: {}, normal: {}]", self.p, self.d, self.n)
    }
}

