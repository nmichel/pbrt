use std::sync::Arc;

use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, Intersection, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use crate::shapes::Shape;

pub struct Union {
    elements: Vec<Arc<dyn Shape>>,
}

impl Union {
    pub fn new(elements: Vec<Arc<dyn Shape>>) -> Union {
        Union { elements }
    }
}

impl Shape for Union {}

impl Intersectable for Union {
    /// The boundary of A ∪ B: every member's boundary, minus what lies inside another member.
    ///
    /// A point where one element's surface enters another is interior to the union, not on its
    /// skin — two overlapping spheres show one silhouette, and the two arcs buried inside the other
    /// sphere are not part of it. Hence the test against *the other* elements, and only them: a
    /// point is always on the boundary of the element that produced it, and asking that element
    /// whether it contains its own surface point would discard every hit, the solids being closed.
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        self.elements
            .iter()
            .enumerate()
            .fold(IntersectionResult::new(), |mut current, (index, element)| {
                for collision in element.intersect(ray, near, far) {
                    if !self.is_inside(&collision, index) {
                        current.push(collision)
                    }
                }

                current.sort_by(|a, b| a.d.partial_cmp(&b.d).unwrap());
                current
            })
    }

    /// A point is inside a union as soon as it is inside **one** element.
    ///
    /// The mirror of `Intersection::contain_point`, which has to consult them all. An empty union
    /// contains nothing, which `any` gives for free.
    fn contain_point(&self, point: &Vector3f) -> bool {
        self.elements.iter().any(|element| element.contain_point(point))
    }
}

impl AABound for Union {
    /// The box containing every element's box, since A ∪ B reaches wherever either does.
    ///
    /// An empty union reports a degenerate box at the origin rather than `AABoundingBox::empty()`:
    /// the empty box is the identity of this very union, and handing it out as a *bound* would give
    /// the accelerator inverted bounds to traverse.
    fn get_bounding_box(&self) -> AABoundingBox {
        match &self.elements[..] {
            &[] => AABoundingBox::new(&Vector3f::zero(), &Vector3f::zero()),

            &[ref first_element, ref other_elements @ ..] => {
                let mut res_bbox = first_element.get_bounding_box();
                for next_element in other_elements.iter() {
                    let bbox = next_element.get_bounding_box();
                    res_bbox.combine_with(&bbox);
                }
                res_bbox
            }
        }
    }
}

impl Union {
    /// Whether `intersection` lies inside any element other than the one at `from`.
    ///
    /// The exclusion is an index rather than the element itself: what has to be skipped is one
    /// *position* in the list, and comparing shapes would mean comparing `Arc` identities — which
    /// is also wrong the day two positions hold the same shape.
    fn is_inside(&self, intersection: &Intersection, from: usize) -> bool {
        for (index, element) in self.elements.iter().enumerate() {
            if index == from {
                continue;
            }

            if element.contain_point(&intersection.p) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::geom::transform::Transform;
    use crate::shapes::{Sphere, Transformed};

    const EPSILON: f64 = 1e-12;

    /// Two spheres side by side, each placed inside the assembly's own frame — the shape a
    /// `csg union { elem … transform { … } … }` builds.
    fn two_spheres() -> Union {
        let placed = |x: f64| -> Arc<dyn Shape> {
            Arc::new(Transformed::new(
                Arc::new(Sphere::new(1.0)),
                Box::new(Transform::translation(Vector3f::new(x, 0.0, 0.0))),
            ))
        };

        Union::new(vec![placed(-1.5), placed(1.5)])
    }

    /// An assembly placed by the loader's current transformation matrix moves **once**.
    ///
    /// This is the trap the module header describes: an `elem`'s transformation and the matrix the
    /// loader carries position different things, and composing them would displace the assembly
    /// twice. The two levels of placement are nested here exactly as the loader nests them — the
    /// elements inside the union, the union inside a `Transformed` — and the assertion is that the
    /// far sphere's outer face is where one displacement puts it, not two.
    #[test]
    fn test_a_placed_assembly_moves_once() {
        const DISPLACEMENT: f64 = 10.0;
        let placed_union = Transformed::new(
            Arc::new(two_spheres()),
            Box::new(Transform::translation(Vector3f::new(DISPLACEMENT, 0.0, 0.0))),
        );

        // Along the x axis, through both spheres: the assembly spans [-2.5, 2.5] in its own frame,
        // so placed once it spans [7.5, 12.5] and placed twice it would span [17.5, 22.5].
        let ray = Ray::new(&Vector3f::new(-100.0, 0.0, 0.0), &Vector3f::new(1.0, 0.0, 0.0));
        let hits = placed_union.intersect(&ray, 0.0, 1000.0);

        assert_eq!(hits.len(), 4, "two spheres, entered and left");
        assert!(
            (hits[0].p.x - (DISPLACEMENT - 2.5)).abs() < EPSILON,
            "the near face is at x = {}, so the assembly was displaced {} times",
            hits[0].p.x,
            hits[0].p.x / DISPLACEMENT
        );
        assert!((hits[3].p.x - (DISPLACEMENT + 2.5)).abs() < EPSILON, "far face at x = {}", hits[3].p.x);
    }

    /// The surface buried inside another element is not part of the union's boundary: the two
    /// spheres of `two_spheres` are apart, so a ray through both reports four crossings, while the
    /// same ray through overlapping spheres reports only the two outer ones.
    #[test]
    fn test_a_buried_surface_is_not_on_the_boundary() {
        let ray = Ray::new(&Vector3f::new(-100.0, 0.0, 0.0), &Vector3f::new(1.0, 0.0, 0.0));

        assert_eq!(
            two_spheres().intersect(&ray, 0.0, 1000.0).len(),
            4,
            "disjoint spheres keep all four crossings"
        );

        let overlapping = Union::new(vec![
            Arc::new(Transformed::new(
                Arc::new(Sphere::new(1.0)),
                Box::new(Transform::translation(Vector3f::new(-0.5, 0.0, 0.0))),
            )),
            Arc::new(Transformed::new(
                Arc::new(Sphere::new(1.0)),
                Box::new(Transform::translation(Vector3f::new(0.5, 0.0, 0.0))),
            )),
        ]);

        let hits = overlapping.intersect(&ray, 0.0, 1000.0);
        assert_eq!(hits.len(), 2, "the two inner arcs are interior to the union");
        assert!((hits[0].p.x - -1.5).abs() < EPSILON, "near face at x = {}", hits[0].p.x);
        assert!((hits[1].p.x - 1.5).abs() < EPSILON, "far face at x = {}", hits[1].p.x);
    }
}
