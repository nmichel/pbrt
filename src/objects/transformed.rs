use std::sync::Arc;

use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::transform::{Transform, Transformable};
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;

use super::Object;

pub struct Transformed {
    pub object: Arc<dyn Object>,
    pub transform: Box<Transform>,
}

impl Transformed {
    pub fn new(object: Arc<dyn Object>, transform: Box<Transform>) -> Self {
        Self {
            object: Arc::clone(&object),
            transform: transform,
        }
    }
}

impl Object for Transformed {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Interaction> {
        let local_ray: Ray = self.transform.transform_ray_to_local(&ray);
        match Object::intersect(self.object.as_ref(), &local_ray, near, far) {
            None => None,
            Some(ref interaction) => {
                Some(Interaction {
                    intersection: self.transform.transform_interaction_to_world(&interaction.intersection),
                    material: interaction.material.clone(),
                })
            }
        }
    }
}

impl Intersectable for Transformed {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        let local_ray: Ray = self.transform.transform_ray_to_local(&ray);
        let mut res: IntersectionResult = IntersectionResult::new();

        for intersection in Intersectable::intersect(self.object.as_ref(), &local_ray, near, far).iter() {
            res.push(self.transform.transform_interaction_to_world(&intersection));
        }

        res
    }

    fn contain_point(&self, point: &Vector3f) -> bool {
        let local_point = self.transform.transform_point_to_local(&point);
        self.object.contain_point(&local_point)
    }
}

impl AABound for Transformed {
    fn get_bounding_box(&self) -> AABoundingBox {
        self.object.get_bounding_box().transform(&self.transform)
    }
}
