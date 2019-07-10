use super::ray::Ray;
use super::vector3::Vector3f;

#[derive(Debug)]
pub struct Intersection {
    p: Vector3f,
    d: f64,
    n: Vector3f
}

pub trait Intersectable {
    fn intersect(&self, ray: &Ray) -> Vec<Intersection>;
}
