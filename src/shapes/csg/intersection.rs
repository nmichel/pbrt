use crate::geom::intersectable::{Intersectable, Intersection};
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;

use super::Elem;

pub struct CSGIntersection {
    elements: Vec<Box<Elem>>
}

impl CSGIntersection {
    pub fn new(elements: Vec<Box<Elem>>) -> CSGIntersection {
        Self { elements }
    }
}

impl Intersectable for CSGIntersection {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Intersection> {
        self.elements.iter().fold(None, |current, e| {
            let local_ray = e.transform.transform_ray_to_local(&ray); // transform ray in the tested elem frame
            let new = e.shape.intersect(&local_ray, near, far);
            match new {
                None =>
                    current,
                Some(local_intersection) => {
                    let new_intersection = e.transform.transform_interaction_to_world(&local_intersection); // transform intersection back in world frame
                    // println!("* local_intersection) pos {:?}", &local_intersection.p);
                    // println!("* new_intersection pos {:?}", &new_intersection.p);
                    let s: *const &Intersectable = &e.shape.as_ref();
                    // println!("* s {:?}", &s);
                    if ! self.is_inside(&new_intersection, s) {                        
                        current
                    }
                    else {
                        // println!("* intersection is INSIDE");
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

impl CSGIntersection {
    fn is_inside(&self, intersection: &Intersection, exclude: *const &Intersectable) -> bool {
        // println!("* exluded is {:?}", exclude);
        for elem in &self.elements {
            let current = &elem.shape.as_ref() as *const &Intersectable;
            // println!("* testing current {:?}", current);
            if current == exclude {
                // println!("* exclude {:?}", &(elem.as_ref() as *const Elem));
                continue;
            }

            let local_p = elem.transform.transform_point_to_local(&intersection.p);
            // println!("Local P {:?}", &local_p);
            if ! elem.shape.contain_point(&local_p) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::ray::Ray;
    use crate::geom::transform::*;
    use crate::geom::vector3;
    use crate::geom::vector3::Vector3f;
    use crate::shapes::csg;
    use crate::shapes::Plane;

    #[test]
    fn test_intersect() {
        let elements = vec![
            Box::new(csg::Elem { shape: Box::new(Plane::new()), transform: Box::new(Transform::translation(Vector3f::new(0.0, 2.0, 0.0))) }), // top
            Box::new(csg::Elem { shape: Box::new(Plane::new()), transform: Box::new(Transform::translation(Vector3f::new(2.0, 0.0, 0.0)) * Transform::rotation_z(-std::f64::consts::PI/2.0)) }), // left
            ];

        let o = CSGIntersection::new(elements);
        let position = Vector3f::new(0.0, 3.0, 30.0);
        let look_at = Vector3f::new(3.0, 3.0, 0.0);
        let direction = vector3::normalize(&(&look_at - &position));
        let ray = Ray::new(&position, &direction);

        match o.intersect(&ray, 0.0, 1000.0) {
            None => println!("NONE"),
            Some(interaction) =>  println!("Point : {:?} {:?}", &interaction.p, &interaction.d)
       }
    }
}
