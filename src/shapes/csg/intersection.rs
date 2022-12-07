use crate::geom::intersectable::Intersectable;
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;

use super::Elem;

pub struct Intersection {
    elements: Vec<Box<Elem>>
}

impl Intersection {
    pub fn new(elements: Vec<Box<Elem>>) -> Intersection {
        Self { elements }
    }
}

impl Intersectable for Intersection {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<crate::geom::intersectable::Intersection> {
        self.elements.iter().fold(None, |current, e| {
            // transform ray in the tested elem frame
            let local_ray = e.transform.transform_ray_to_local(&ray);

            // Test the current element for intersection with the ray 
            let new = e.shape.intersect(&local_ray, near, far);
            match new {
                None =>
                    current,
                Some(local_intersection) => {
                    // transform intersection back in world frame
                    let new_intersection = e.transform.transform_interaction_to_world(&local_intersection);

                    // If the intersection point is not inside ALL OTHER elements, ignore it
                    if ! self.is_inside(&new_intersection, e.as_ref()) {
                        current
                    }
                    else {
                        match &current {
                            None =>
                                // No collision had been found until now
                                Some(new_intersection),

                            Some(current_intersection) =>
                                // A collision has already been found
                                // Compare the distance to ray origin of the new collision.
                                // If nearer, keep it; otherwise keep the current one.
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

impl Intersection {
    fn is_inside(&self, intersection: &crate::geom::intersectable::Intersection, exclude: &Elem) -> bool {
        for elem in &self.elements {
            let current = elem.as_ref() as *const Elem;
            if current == exclude {
                continue;
            }

            let local_p = elem.transform.transform_point_to_local(&intersection.p);
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
            Box::new(csg::Elem { shape: Box::new(Plane::new()), transform: Box::new(Transform::translation(Vector3f::new(2.0, 0.0, 0.0)) * Transform::rotation_z(-std::f64::consts::PI/2.0)) }), // left
            Box::new(csg::Elem { shape: Box::new(Plane::new()), transform: Box::new(Transform::translation(Vector3f::new(0.0, 2.0, 0.0))) }), // top
            ];

        let o = Intersection::new(elements);
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
