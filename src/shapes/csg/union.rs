use crate::geom::intersectable::{Intersectable, Intersection};
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
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Intersection> {
        self.elements.iter().fold(None, |current, e| {
            let local_ray = e.transform.transform_ray_to_local(&ray); // transform ray in the tested elem frame
            let new = e.shape.intersect(&local_ray, near, far);
            match new {
                None =>
                    current,
                Some(intersection) => {
                    let new_intersection = e.transform.transform_interaction_to_world(&intersection); // transform intersection back in world frame
                    if self.is_inside(&new_intersection, e.shape.as_ref()) {
                        current
                    }
                    else {
                        match &current {
                            None =>
                                Some(new_intersection),
                            Some(current_intersection) =>
                                if new_intersection.d < current_intersection.d {
                                    Some(new_intersection)
                                }
                                else {
                                    current
                                }
                        }
                    }
                }
            }
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
    fn is_inside(&self, intersection: &Intersection, exclude: &Intersectable) -> bool {
        for elem in &self.elements {
            let current: *const Intersectable = elem.shape.as_ref();
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


