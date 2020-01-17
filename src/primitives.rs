use std::rc::Rc;
use super::geom::intersectable::{Intersectable, Intersection};
use super::geom::ray::Ray;
use super::geom::transform::Transform;
use super::materials::material::Material;

pub struct Primitive {
    pub shape: Box<Intersectable>,
    pub transform: Box<Transform>,
    pub material: Rc<Material>
}

impl Primitive {
    pub fn new(shape: Box<Intersectable>, transform: Box<Transform>, material: Rc<Material>) -> Self {
        Self { shape, transform, material }
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
