use crate::geom::intersectable::{IntersectionResult, Intersectable, Intersection};
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use super::Elem;

pub struct Union {
    elements: Vec<Box<Elem>>
}

impl Union {
    pub fn new(elements: Vec<Box<Elem>>) -> Union {
        Union { elements }
    }
}

impl Intersectable for Union {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        self.elements.iter().fold(IntersectionResult::new(), |mut current, e| {
            // transform ray in the tested elem frame
            let local_ray = e.transform.transform_ray_to_local(&ray);

            // Search intersections with the current element
            let element_collisions = e.shape.intersect(&local_ray, near, far);

            for collision in element_collisions.iter() {
                // Transform collision back in world frame
               let collision_in_world_space = e.transform.transform_interaction_to_world(&collision);

               // Each collision that doesn't lie inside any other element's volume is kept in the result set
               if !self.is_inside(&collision_in_world_space, e.as_ref()) {
                   current.push(collision_in_world_space)
               }
           }

           current.sort_by(|a, b| a.d.partial_cmp(&b.d).unwrap());
           current
        })
    }

    fn contain_point(&self, point: &Vector3f) -> bool {
        for elem in &self.elements {
            let local_point = elem.transform.transform_point_to_local(&point);
            if elem.shape.contain_point(&local_point) {
                return true;
            }
        }

        false
    }
}

impl Union {
    fn is_inside(&self, intersection: &Intersection, exclude: &Elem) -> bool {
        for elem in &self.elements {
            let current = elem.as_ref() as *const Elem;
            if current == exclude {
                continue;
            }

            let local_p = elem.transform.transform_point_to_local(&intersection.p);
            if elem.shape.contain_point(&local_p) {
                return true;
            }
        }

        false
    }
}


