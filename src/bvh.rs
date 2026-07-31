use std::cmp::Ordering;
use std::fmt;

use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::ray::Ray;
use crate::utils::random_double;

pub struct BVHNode<T>
where
    T: AABound,
{
    left: Option<Box<BVHNode<T>>>,
    right: Option<Box<BVHNode<T>>>,
    aabbox: AABoundingBox,
    primitives: Vec<T>,
}

pub trait Accumulator<T> {
    fn accumulate(&mut self, items: &mut Vec<T>) -> ();
}

impl<T: AABound + Clone> BVHNode<T> {
    /// Builds a subtree over `primitives`, which is consumed as the recursion partitions it.
    ///
    /// # Precondition: `primitives` must not be empty
    ///
    /// A tree over nothing is not a thing this type can represent — its root would need a
    /// bounding box, and the empty box is not one a traversal may be given. Emptiness belongs
    /// one level up, in the `Option` that `Scene::build_bvh` returns, so it is asserted here
    /// rather than handled. Same reasoning as `AABoundingBox::hit`: "does a ray hit nothing" and
    /// "what does a tree over nothing look like" are questions with no useful answer.
    ///
    /// The assert also replaces a crash. On an empty vector the `match` below fell to its `_`
    /// arm, `len() / 2` was 0, `drain(0..)` moved everything — that is, nothing — to the right,
    /// and the left side recursed on the same empty vector until the stack overflowed. Measured:
    /// `fatal runtime error: stack overflow`, SIGABRT.
    pub fn new(primitives: &mut Vec<T>) -> Self {
        debug_assert!(
            !primitives.is_empty(),
            "BVHNode::new needs at least one primitive; an empty scene is a `None` tree, see Scene::build_bvh"
        );

        // Sort primitive with respect to a comparator randomly chosen
        primitives.sort_by(Self::choose_comparator());

        // Depending on the count of elements in vector of primitive
        // Either create BVHNode childrne, or move the primitive
        // in the current node.
        match &primitives[..] {
            // One element, move primitive into current node
            [elem] => {
                Self {
                    left: None,
                    right: None,
                    aabbox: elem.get_bounding_box(),
                    primitives: primitives[..].to_vec(),
                }
            }
            // Several elements, split them among 2 sub nodes
            _ => {
                let half_len = primitives.len() / 2;
                let mut right_half: Vec<_> = primitives.drain(half_len..).collect();
                let left_node = BVHNode::new(primitives);
                let right_node = BVHNode::new(&mut right_half);
                Self {
                    aabbox: AABoundingBox::combine(&left_node.aabbox, &right_node.aabbox),
                    left: Some(Box::new(left_node)),
                    right: Some(Box::new(right_node)),
                    primitives: vec![],
                }
            }
        }
    }

    pub fn query(&self, ray: &Ray, near: f64, far: f64, accumulator: &mut dyn Accumulator<T>) -> () {
        if self.aabbox.hit(ray, near, far).is_none() {
            return;
        }

        if self.primitives.len() > 0 {
            accumulator.accumulate(&mut self.primitives.clone())
        }
        else {
            Self::query_subnode(&self.left, ray, near, far, accumulator);
            Self::query_subnode(&self.right, ray, near, far, accumulator);
        }
    }

    fn query_subnode<'a>(node: &'a Option<Box<BVHNode<T>>>, ray: &Ray, near: f64, far: f64, accumulator: &mut dyn Accumulator<T>) -> () {
        match &node {
            None => (),
            Some(node) => node.query(ray, near, far, accumulator),
        }
    }

    fn choose_comparator() -> fn(&T, &T) -> Ordering {
        let r = random_double() * 3.0;
        if r < 1.0 {
            Self::compare_x
        }
        else if r < 2.0 {
            Self::compare_y
        }
        else {
            Self::compare_z
        }
    }

    fn compare_x(a: &T, b: &T) -> Ordering {
        a.get_bounding_box().bmin.x.partial_cmp(&b.get_bounding_box().bmin.x).unwrap()
    }

    fn compare_y(a: &T, b: &T) -> Ordering {
        a.get_bounding_box().bmin.y.partial_cmp(&b.get_bounding_box().bmin.y).unwrap()
    }

    fn compare_z(a: &T, b: &T) -> Ordering {
        a.get_bounding_box().bmin.z.partial_cmp(&b.get_bounding_box().bmin.z).unwrap()
    }
}

impl<T> fmt::Display for BVHNode<T>
where
    T: AABound,
{
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "* aabbox {:?}", &self.aabbox)?;
        writeln!(f, "primitives {}", self.primitives.len())?;
        match &self.left {
            None => {
                writeln!(f, "No left child")?;
            }
            Some(sub_node) => {
                writeln!(f, "left child \n{}", *sub_node)?;
            }
        }
        match &self.right {
            None => {
                writeln!(f, "No right child")?;
            }
            Some(sub_node) => {
                writeln!(f, "right child \n{}", *sub_node)?;
            }
        }

        fmt::Result::Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::vector3::Vector3f;

    /// The smallest `AABound + Clone` there is: a bounding box standing in for a primitive.
    ///
    /// The tree is generic and knows nothing about what it holds beyond its box — that is the
    /// whole point of the `AABound` seam — so nothing else is needed here. Testing with spheres
    /// and materials would only prove that the seam had been crossed.
    #[derive(Clone)]
    struct Boxed(AABoundingBox);

    impl AABound for Boxed {
        fn get_bounding_box(&self) -> AABoundingBox {
            self.0
        }
    }

    /// A unit box whose lower x corner sits at `x`.
    fn unit_box_at(x: f64) -> Boxed {
        Boxed(AABoundingBox::new(&Vector3f::new(x, 0.0, 0.0), &Vector3f::new(x + 1.0, 1.0, 1.0)))
    }

    /// One primitive is a leaf: it keeps the primitive and has no children.
    #[test]
    fn test_single_primitive_is_a_leaf() {
        let mut primitives = vec![unit_box_at(0.0)];
        let tree = BVHNode::new(&mut primitives);

        assert_eq!(tree.primitives.len(), 1);
        assert!(tree.left.is_none() && tree.right.is_none());
    }

    /// Building must terminate, and the root box must enclose every primitive.
    ///
    /// The termination half is not idle: the split axis is drawn at random, so this exercises a
    /// different partition on every run.
    #[test]
    fn test_build_encloses_every_primitive() {
        let mut primitives: Vec<Boxed> = (0..7).map(|i| unit_box_at(i as f64 * 3.0)).collect();

        // `new` drains the vector, so the expected bound is taken first.
        let expected = primitives.iter().fold(AABoundingBox::empty(), |mut acc, primitive| {
            acc.combine_with(&primitive.get_bounding_box());
            acc
        });

        let tree = BVHNode::new(&mut primitives);

        assert_eq!(tree.aabbox.bmin, expected.bmin);
        assert_eq!(tree.aabbox.bmax, expected.bmax);
    }
}
