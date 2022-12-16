use crate::geom::intersectable::{Intersectable, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;

use super::Elem;

pub struct Substraction {
    elements: Vec<Box<Elem>>,
}

impl Substraction {
    pub fn new(elements: Vec<Box<Elem>>) -> Self {
        Self { elements }
    }
}

impl Intersectable for Substraction {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        match &self.elements[..] {
            &[] => IntersectionResult::new(),

            &[ref base_element, ref substracted_elements @ ..] => {
                let mut res = IntersectionResult::new();

                // transform ray in the base elem frame
                let local_ray = base_element.transform.transform_ray_to_local(&ray);

                // Search intersections with this base element
                let base_element_collisions = base_element.shape.intersect(&local_ray, near, far);

                for collision in base_element_collisions.iter() {
                    // Transform collision back in world frame
                    let collision_in_world_space = base_element.transform.transform_interaction_to_world(&collision);

                    // Keep collision only it doesn't belong to a substracted volume.
                    if !self.is_point_in_substracted(&collision_in_world_space.p, base_element) {
                        res.push(collision_in_world_space)
                    }
                }

                for elem in substracted_elements {
                    // transform ray in the current elem frame
                    let local_ray = elem.transform.transform_ray_to_local(&ray);

                    // Search intersections with this element
                    let local_collision = elem.shape.intersect(&local_ray, near, far);

                    for collision in local_collision.iter() {
                        // Transform collision back in world frame
                        let mut collision_in_world_space = elem.transform.transform_interaction_to_world(&collision);

                        let local_point = base_element.transform.transform_point_to_local(&collision_in_world_space.p);
                        if base_element.shape.contain_point(&local_point) {
                            // Keep collision only it doesn't belong to a substracted volume.
                            if !self.is_point_in_substracted(&collision_in_world_space.p, elem.as_ref()) {
                                collision_in_world_space.n.mul_to_me(-1.0);
                                res.push(collision_in_world_space)
                            }
                        }
                    }
                }

                res.sort_by(|a, b| a.d.partial_cmp(&b.d).unwrap());
                res
            }
        }
    }

    fn contain_point(&self, point: &Vector3f) -> bool {
        match &self.elements[..] {
            &[] => false,

            &[ref base_element, ref substracted_elements @ ..] => {
                let local_point = base_element.transform.transform_point_to_local(&point);
                if !base_element.shape.contain_point(&local_point) {
                    return false;
                }

                for elem in substracted_elements {
                    let local_point = elem.transform.transform_point_to_local(&point);
                    if elem.shape.contain_point(&local_point) {
                        return false;
                    }
                }

                true
            }
        }
    }
}

impl Substraction {
    fn is_point_in_substracted(&self, point: &Vector3f, exclude: &Elem) -> bool {
        match &self.elements[..] {
            &[] => false,

            &[_, ref substracted_elements @ ..] => {
                for elem in substracted_elements {
                    let current = elem.as_ref() as *const Elem;
                    if current == exclude {
                        continue;
                    }

                    let local_point = elem.transform.transform_point_to_local(&point);
                    if elem.shape.contain_point(&local_point) {
                        return true;
                    }
                }

                false
            }
        }
    }
}
