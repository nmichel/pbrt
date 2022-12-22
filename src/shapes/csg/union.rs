use super::Elem;
use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, Intersection, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::transform::Transformable;
use crate::geom::vector3::Vector3f;

pub struct Union {
    elements: Vec<Box<Elem>>,
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

impl AABound for Union {
    fn get_bounding_box(&self) -> AABoundingBox {
        match &self.elements[..] {
            &[] => AABoundingBox::new(&Vector3f::zero(), &Vector3f::zero()),

            &[ref first_element, ref other_elements @ ..] => {
                let mut res_bbox = first_element.shape.get_bounding_box().transform(&first_element.transform);
                for next_element in other_elements.iter() {
                    let bbox = next_element.shape.get_bounding_box().transform(&next_element.transform);
                    res_bbox.combine_with(&bbox);
                }
                res_bbox
            }
        }
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
