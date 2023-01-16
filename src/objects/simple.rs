use std::sync::Arc;

use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, Intersection, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::materials::Material;
use crate::shapes::Shape;

use super::Object;

pub struct Simple {
    pub shape: Arc<dyn Shape>,
    pub material: Arc<dyn Material>,
}

impl Simple {
    pub fn new(shape: Arc<dyn Shape>, material: Arc<dyn Material>) -> Self {
        Self {
            shape: Arc::clone(&shape),
            material: Arc::clone(&material),
        }
    }
}

impl Object for Simple {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Interaction> {
        let intersections = self.shape.intersect(ray, near, far);
        if intersections.len() > 0 {
            let intersection: *const Intersection = intersections.as_ptr();
            let material = self.material.clone();
            Some(Interaction {
                intersection: unsafe { *intersection },
                material,
            })
        }
        else {
            None
        }
    }
}

impl Intersectable for Simple {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        self.shape.intersect(ray, near, far)
    }

    fn contain_point(&self, point: &Vector3f) -> bool {
        self.shape.contain_point(point)
    }
}

impl AABound for Simple {
    fn get_bounding_box(&self) -> AABoundingBox {
        self.shape.get_bounding_box()
    }
}
