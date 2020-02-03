use super::geom::intersectable::{Intersectable, Intersection};
use super::geom::ray::Ray;
use super::geom::transform::Transform;
use super::materials::Material;
use std::sync::Arc;

pub struct Primitive {
    pub shape: Box<Intersectable>,
    pub transform: Box<Transform>,
    pub material: Arc<Material>
}

impl Primitive {
    pub fn new(shape: Box<Intersectable>, transform: Box<Transform>, material: Arc<Material>) -> Self {
        Self { shape, transform, material: Arc::clone(&material) }
    }
}

impl Intersectable for Primitive {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Intersection> {
        let local_ray = self.transform.transform_ray_to_local(&ray);
        match self.shape.intersect(&local_ray, near, far) {
            Some(intersection) => Some(self.transform.transform_interaction_to_world(&intersection)),
            None => None
        }
    }
}
