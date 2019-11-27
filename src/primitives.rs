use super::geom::intersectable::{Intersectable, Intersection};
use super::geom::ray::Ray;
use super::geom::transform::Transform;

pub struct Primitive {
    shape: Box<Intersectable>,
    transform: Box<Transform>
}

impl Primitive {
    /// Build a new `Primitive` by composing an `Intersectable` and a `Transform`.
    /// 
    pub fn new(shape: Box<Intersectable>, transform: Box<Transform>) -> Self {
        Self { shape, transform }
    }
}

impl Intersectable for Primitive {
    fn intersect(&self, ray: &Ray) -> Option<Intersection> {
        let local_ray = self.transform.transform_ray_to_local(&ray);
        match self.shape.intersect(&local_ray) {
            Some(intersection) => Some(self.transform.transform_interaction_to_world(&intersection)),
            None => None
        }
    }
}
