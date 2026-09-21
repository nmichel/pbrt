use std::sync::Arc;

use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use crate::shapes::Shape;

pub struct Intersection {
    elements: Vec<Arc<dyn Shape>>,
}

impl Shape for Intersection {}

impl Intersection {
    pub fn new(elements: Vec<Arc<dyn Shape>>) -> Intersection {
        Self { elements }
    }
}

impl Intersectable for Intersection {
    /// The boundary of A ∩ B: the part of each member's boundary that lies inside all the others.
    ///
    /// The exact complement of the union's rule, and the same argument the other way round: a
    /// surface point of A outside B is outside the intersection altogether, while one inside B is
    /// where the two solids share their skin. A sphere cut by a plane keeps the cap below the plane
    /// and drops the rest.
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        self.elements
            .iter()
            .enumerate()
            .fold(IntersectionResult::new(), |mut current, (index, element)| {
                for collision in element.intersect(ray, near, far) {
                    if self.is_inside(&collision, index) {
                        current.push(collision)
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

        self.elements.iter().all(|element| element.contain_point(point))
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
                let mut res_bbox = first_element.get_bounding_box();
                for next_element in other_elements.iter() {
                    let bbox = next_element.get_bounding_box();
                    res_bbox.intersect_with(&bbox);
                }
                res_bbox
            }
        }
    }
}

impl Intersection {
    /// Whether `intersection` lies inside every element other than the one at `from`.
    ///
    /// The exclusion is an index rather than the element itself: what has to be skipped is one
    /// *position* in the list, and comparing shapes would mean comparing `Arc` identities — which
    /// is also wrong the day two positions hold the same shape.
    fn is_inside(&self, intersection: &crate::geom::intersectable::Intersection, from: usize) -> bool {
        for (index, element) in self.elements.iter().enumerate() {
            if index == from {
                continue;
            }

            if !element.contain_point(&intersection.p) {
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
    use crate::geom::transform::Transform;
    use crate::geom::vector3;
    use crate::geom::vector3::Vector3f;
    use crate::shapes::{Plane, Sphere, Transformed};

    #[test]
    fn test_intersect() {
        let elements: Vec<Arc<dyn Shape>> = vec![
            Arc::new(Transformed::new(
                Arc::new(Plane::new()),
                Box::new(Transform::translation(Vector3f::new(2.0, 0.0, 0.0)) * Transform::rotation_z(-std::f64::consts::PI / 2.0)),
            )), // left
            Arc::new(Transformed::new(
                Arc::new(Plane::new()),
                Box::new(Transform::translation(Vector3f::new(0.0, 2.0, 0.0))),
            )), // top
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

    /// A sphere cut by a plane is bounded, and bounded the same way whichever order it is written
    /// in.
    ///
    /// This is what decides whether `Scene::commit` keeps the shape in the accelerator or puts it on
    /// the list tested for every ray. Reading only the first element's bound makes the answer depend
    /// on the order the two were typed in, which is the sort of thing that silently costs a scene
    /// its acceleration.
    ///
    /// These elements are bare shapes, needing no placement, so the plane brings its own bound — the
    /// half-space y ≤ 0 — and the result is the lower half of the sphere, tightly. A *placed* plane
    /// would bound the same solid more loosely: `AABoundingBox::transform` widens an unbounded box to
    /// infinity on every axis, the y ≤ 0 arriving forgotten, and the result would be the whole
    /// sphere box. Loose and correct, since a bound only owes containment.
    #[test]
    fn test_a_sphere_cut_by_a_plane_is_bounded_either_way() {
        let sphere: Arc<dyn Shape> = Arc::new(Sphere::new(1.0));
        let plane: Arc<dyn Shape> = Arc::new(Plane::new());

        let sphere_first = Intersection::new(vec![Arc::clone(&sphere), Arc::clone(&plane)]);
        let plane_first = Intersection::new(vec![plane, sphere]);

        for (label, shape) in [("sphere first", &sphere_first), ("plane first", &plane_first)] {
            let bbox = shape.get_bounding_box();

            assert!(bbox.is_bounded(), "{}: a half-sphere is a bounded thing", label);
            assert_eq!(bbox.bmin, Vector3f::new(-1.0, -1.0, -1.0), "{}", label);
            assert_eq!(bbox.bmax, Vector3f::new(1.0, 0.0, 1.0), "{}", label);
        }
    }

    /// Every element must contain the point, not just one — which is the union's test.
    #[test]
    fn test_contain_point_requires_every_element() {
        let shape = Intersection::new(vec![Arc::new(Sphere::new(1.0)), Arc::new(Plane::new())]);

        // Inside the sphere and below y = 0: inside both, so inside the intersection.
        assert!(shape.contain_point(&Vector3f::new(0.0, -0.5, 0.0)));

        // Inside the sphere but above the plane — in one element only.
        assert!(!shape.contain_point(&Vector3f::new(0.0, 0.5, 0.0)));

        // Below the plane but outside the sphere — again one element only.
        assert!(!shape.contain_point(&Vector3f::new(5.0, -0.5, 0.0)));
    }
}
