use super::ray::Ray;
use super::vector3::Vector3f;
use std::fmt;

#[derive(Debug)]
pub struct Intersection {
    /// Intersection point
    pub p: Vector3f,

    /// Distance to the `Ray` origin
    pub d: f64,

    /// Normal vector at intersection point
    pub n: Vector3f,

    /// Inverse direction of the `Ray`
    pub wo: Vector3f
}

pub trait Intersectable {
    fn intersect(&self, ray: &Ray) -> Vec<Intersection>;
}

impl fmt::Display for Intersection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[position: {}, distance: {}, normal: {}]", self.p, self.d, self.n)
    }
}

