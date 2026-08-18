use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::transform::Transformable;
use crate::geom::vector3::Vector3f;
use crate::shapes::Shape;

use super::Elem;

pub struct Intersection {
    elements: Vec<Box<Elem>>,
}

impl Shape for Intersection {}

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

    /// A point is inside an intersection when it is inside **every** element.
    ///
    /// Requiring one would be the test for a union. The two differ nowhere more visibly than here:
    /// `Union::contain_point` accepts on the first element that contains the point, this one has to
    /// consult them all before accepting.
    ///
    /// An intersection with no element contains nothing, matching `Union` and `Substraction` — and
    /// not the `all` of an empty iterator, which would be `true` and would make an empty shape
    /// swallow every point.
    fn contain_point(&self, point: &Vector3f) -> bool {
        if self.elements.is_empty() {
            return false;
        }

        self.elements.iter().all(|elem| {
            let local_point = elem.transform.transform_point_to_local(&point);
            elem.shape.contain_point(&local_point)
        })
    }
}

impl AABound for Intersection {
    /// The region shared by every element, which is the tightest bound an intersection admits.
    ///
    /// A ∩ B is contained in A and in B, so it is contained in the intersection of their bounds. The
    /// consequence that matters for the accelerator: an intersection is **bounded as soon as one of
    /// its elements is**, whatever the others do. `Intersection(sphere, plane)` is a half-sphere and
    /// belongs in the tree, even though the plane it is cut by is infinite — and it does so whichever
    /// order the two are written in, since intersecting bounds is commutative.
    ///
    /// Reading only the first element's bound instead would still be conservative, but it would make
    /// the declaration order decide whether the shape is bounded, and `Scene::commit` would push a
    /// perfectly finite half-sphere out of the accelerator on the strength of a typing order.
    fn get_bounding_box(&self) -> AABoundingBox {
        match &self.elements[..] {
            &[] => AABoundingBox::new(&Vector3f::zero(), &Vector3f::zero()),

            &[ref first_element, ref other_elements @ ..] => {
                let mut res_bbox = first_element.shape.get_bounding_box().transform(&first_element.transform);
                for next_element in other_elements.iter() {
                    let bbox = next_element.shape.get_bounding_box().transform(&next_element.transform);
                    res_bbox.intersect_with(&bbox);
                }
                res_bbox
            }
        }
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
    use std::sync::Arc;

    use super::*;
    use crate::geom::ray::Ray;
    use crate::geom::transform::*;
    use crate::geom::vector3;
    use crate::geom::vector3::Vector3f;
    use crate::shapes::{csg, Plane, Sphere};

    #[test]
    fn test_intersect() {
        let elements = vec![
            Box::new(csg::Elem {
                shape: Arc::new(Plane::new()),
                transform: Box::new(Transform::translation(Vector3f::new(2.0, 0.0, 0.0)) * Transform::rotation_z(-std::f64::consts::PI / 2.0)),
            }), // left
            Box::new(csg::Elem {
                shape: Arc::new(Plane::new()),
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

    fn untransformed(shape: Arc<dyn Shape>) -> Box<csg::Elem> {
        Box::new(csg::Elem {
            shape,
            transform: Box::new(Transform::identity()),
        })
    }

    /// A sphere cut by a plane is bounded, and bounded the same way whichever order it is written
    /// in.
    ///
    /// This is what decides whether `Scene::commit` keeps the shape in the accelerator or puts it on
    /// the list tested for every ray. Reading only the first element's bound makes the answer depend
    /// on the order the two were typed in, which is the sort of thing that silently costs a scene
    /// its acceleration.
    ///
    /// The bound is the sphere's, since `AABoundingBox::transform` widens an unbounded box to
    /// infinity on every axis and the plane's half-space arrives with its y ≤ 0 forgotten. Loose by
    /// a factor of two in y, and correct: it contains the half-sphere, which is what a bound owes.
    #[test]
    fn test_a_sphere_cut_by_a_plane_is_bounded_either_way() {
        let sphere_first = Intersection::new(vec![untransformed(Arc::new(Sphere::new(1.0))), untransformed(Arc::new(Plane::new()))]);
        let plane_first = Intersection::new(vec![untransformed(Arc::new(Plane::new())), untransformed(Arc::new(Sphere::new(1.0)))]);

        for (label, shape) in [("sphere first", &sphere_first), ("plane first", &plane_first)] {
            let bbox = shape.get_bounding_box();

            assert!(bbox.is_bounded(), "{}: a half-sphere is a bounded thing", label);
            assert_eq!(bbox.bmin, Vector3f::new(-1.0, -1.0, -1.0), "{}", label);
            assert_eq!(bbox.bmax, Vector3f::new(1.0, 1.0, 1.0), "{}", label);
        }
    }

    /// Every element must contain the point, not just one — which is the union's test.
    #[test]
    fn test_contain_point_requires_every_element() {
        let shape = Intersection::new(vec![untransformed(Arc::new(Sphere::new(1.0))), untransformed(Arc::new(Plane::new()))]);

        // Inside the sphere and below y = 0: inside both, so inside the intersection.
        assert!(shape.contain_point(&Vector3f::new(0.0, -0.5, 0.0)));

        // Inside the sphere but above the plane — in one element only.
        assert!(!shape.contain_point(&Vector3f::new(0.0, 0.5, 0.0)));

        // Below the plane but outside the sphere — again one element only.
        assert!(!shape.contain_point(&Vector3f::new(5.0, -0.5, 0.0)));
    }
}
