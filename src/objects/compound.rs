use std::sync::Arc;

use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;

use super::Object;

pub struct Compound {
    pub objects: Vec<Arc<dyn Object>>,
}

impl Compound {
    pub fn new(objects: &Vec<Arc<dyn Object>>) -> Self {
        Self { objects: objects.clone() }
    }
}

impl Object for Compound {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Interaction> {
        self.objects.iter().fold(Option::<Interaction>::None, |acc, object| {
            match (acc, Object::intersect(object.as_ref(), &ray, near, far)) {
                (acc, None) => acc,
                (None, interaction) => interaction,
                (Some(prev_interaction), Some(interaction)) => {
                    if interaction.intersection.d < prev_interaction.intersection.d {
                        Some(interaction)
                    }
                    else {
                        Some(prev_interaction)
                    }
                }
            }
        })
    }
}

impl Intersectable for Compound {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        self.objects.iter().fold(IntersectionResult::new(), |mut acc, object| {
            let mut res = Intersectable::intersect(object.as_ref(), &ray, near, far);
            acc.append(&mut res);
            acc
        })
    }

    fn contain_point(&self, point: &Vector3f) -> bool {
        for object in self.objects.iter() {
            if object.contain_point(&point) {
                return true;
            }
        }
        return false;
    }
}

impl AABound for Compound {
    fn get_bounding_box(&self) -> AABoundingBox {
        self.objects.iter().fold(AABoundingBox::empty(), |mut acc, object| {
            acc.combine_with(&mut object.get_bounding_box());
            acc
        })
    }
}
