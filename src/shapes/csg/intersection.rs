use crate::geom::intersectable::{Intersectable, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;

use super::Elem;

pub struct Intersection {
    elements: Vec<Box<Elem>>,
}

impl Intersection {
    pub fn new(elements: Vec<Box<Elem>>) -> Intersection {
        Self { elements }
    }
}

impl Intersectable for Intersection {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        self.elements.iter().fold(IntersectionResult::new(), |mut current, e| {
            // transform ray in the tested elem frame
            let local_ray = e.transform.transform_ray_to_local(&ray);

            // Search intersections with the current element
            let element_collisions = e.shape.intersect(&local_ray, near, far);

            for collision in element_collisions.iter() {
                // Transform collision back in world frame
                let collision_in_world_space = e.transform.transform_interaction_to_world(&collision);

                // Each collision that lies inside all other element's volume is kept in the result set
                if self.is_inside(&collision_in_world_space, e.as_ref()) {
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

impl Intersection {
    fn is_inside(&self, intersection: &crate::geom::intersectable::Intersection, exclude: &Elem) -> bool {
        for elem in &self.elements {
            let current = elem.as_ref() as *const Elem;
            if current == exclude {
                continue;
            }

            let local_p = elem.transform.transform_point_to_local(&intersection.p);
            if !elem.shape.contain_point(&local_p) {
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
    use crate::shapes::{csg, Plane};

    #[test]
    fn test_intersect() {
        let elements = vec![
            Box::new(csg::Elem {
                shape: Box::new(Plane::new()),
                transform: Box::new(Transform::translation(Vector3f::new(2.0, 0.0, 0.0)) * Transform::rotation_z(-std::f64::consts::PI / 2.0)),
            }), // left
            Box::new(csg::Elem {
                shape: Box::new(Plane::new()),
                transform: Box::new(Transform::translation(Vector3f::new(0.0, 2.0, 0.0))),
            }), // top
        ];

        let o = Intersection::new(elements);
        let position = Vector3f::new(0.0, 3.0, 30.0);
        let look_at = Vector3f::new(3.0, 3.0, 0.0);
        let direction = vector3::normalize(&(&look_at - &position));
        let ray = Ray::new(&position, &direction);

        match o.intersect(&ray, 0.0, 1000.0).as_slice() {
            [] => println!("NONE"),
            [ref interaction, ..] => println!("Point : {:?} {:?}", &interaction.p, &interaction.d),
        }
    }
}
