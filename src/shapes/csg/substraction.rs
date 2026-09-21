use std::sync::Arc;

use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use crate::shapes::Shape;

pub struct Substraction {
    elements: Vec<Arc<dyn Shape>>,
}

impl Shape for Substraction {}

impl Substraction {
    pub fn new(elements: Vec<Arc<dyn Shape>>) -> Self {
        Self { elements }
    }
}

impl Intersectable for Substraction {
    /// The boundary of A ∖ (B ∪ C ∖ …): the first element's skin outside every other, plus the
    /// others' skin where it lies inside the first.
    ///
    /// The only operation whose elements are not interchangeable — the first is the base, the rest
    /// are removed from it — and the only one whose result carries a surface that belongs to no
    /// element as such: where B bites into A, the visible skin is B's, seen **from inside** B. Hence
    /// the flipped normal, the one place an operation touches an interaction rather than just
    /// keeping or dropping it.
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        match &self.elements[..] {
            &[] => IntersectionResult::new(),

            &[ref base_element, ref substracted_elements @ ..] => {
                let mut res = IntersectionResult::new();

                for collision in base_element.intersect(ray, near, far) {
                    // Keep collision only it doesn't belong to a substracted volume.
                    if !self.is_point_in_substracted(&collision.p, 0) {
                        res.push(collision)
                    }
                }

                for (index, element) in substracted_elements.iter().enumerate() {
                    // The elements being the tail of the list, the position in `self.elements` is
                    // one further along — which is what `is_point_in_substracted` excludes.
                    let position = index + 1;

                    for mut collision in element.intersect(ray, near, far) {
                        if base_element.contain_point(&collision.p) {
                            // Keep collision only it doesn't belong to a substracted volume.
                            if !self.is_point_in_substracted(&collision.p, position) {
                                collision.n.mul_to_me(-1.0);
                                res.push(collision)
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
                if !base_element.contain_point(point) {
                    return false;
                }

                for element in substracted_elements {
                    if element.contain_point(point) {
                        return false;
                    }
                }

                true
            }
        }
    }
}

impl AABound for Substraction {
    /// Return the bounding box of the first element (from which others are
    /// substracted).
    /// In many case it's oversized, but I postpone finding a better answer
    /// as it's a first acceptable approximation.
    ///
    fn get_bounding_box(&self) -> AABoundingBox {
        match &self.elements[..] {
            &[] => AABoundingBox::new(&Vector3f::zero(), &Vector3f::zero()),

            &[ref first_element, ref _other_elements @ ..] => first_element.get_bounding_box(),
        }
    }
}

impl Substraction {
    /// Whether `point` has been carved away by an element other than the one at `from`.
    ///
    /// The base element is never consulted: it is what the others are removed *from*, so being
    /// inside it is the very condition for a point to be kept. Passing `0` as `from` therefore
    /// excludes nothing, the base not being among the elements this walks.
    fn is_point_in_substracted(&self, point: &Vector3f, from: usize) -> bool {
        self.elements
            .iter()
            .enumerate()
            .skip(1)
            .any(|(index, element)| index != from && element.contain_point(point))
    }
}
