use std::cmp::Ordering;
use std::fmt;

use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::ray::Ray;

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

/// Work counters for traversals of the scene accelerator.
///
/// Same purpose as [`crate::shapes::triangle_mesh::TraversalStats`], and deliberately a separate
/// type: the work units differ — objects tested here, triangles there — and the two traversals
/// are not yet the same algorithm. Merging them now would abstract over a difference that still
/// exists; the moment to do it is when this tree has been flattened and ordered like the mesh's,
/// and not before.
///
/// Reproducible to the unit for a given ray set, a traversal drawing no random numbers. Read as
/// work *per ray*: accumulate over a ray set, then divide by its size.
#[derive(Default, Clone, Copy)]
pub struct TraversalStats {
    /// Nodes whose contents were examined. A node whose box the ray misses is not counted — what
    /// it cost is a box test, which `box_tests` counts.
    pub nodes_visited: usize,

    /// Ray/box tests. Every node entered performs exactly one, on its own box, so this figure
    /// also counts the nodes the traversal reached at all.
    pub box_tests: usize,

    /// Objects handed to `Object::intersect`.
    ///
    /// Today this is also the number of primitives *cloned out of the tree*: the accelerator
    /// returns every candidate it cannot exclude, one allocation and one atomic refcount bump
    /// each, and the scene then tests all of them in whatever order they arrived. The two figures
    /// coincide only because nothing prunes between them — which is the defect, not a property.
    pub object_tests: usize,
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
        debug_assert!(
            primitives.iter().all(|primitive| primitive.get_bounding_box().is_bounded()),
            "BVHNode::new was given an unbounded primitive; those belong outside the accelerator, see Scene::commit"
        );

        let axis = Self::widest_centroid_axis(primitives);
        primitives.sort_by(|a, b| Self::compare_centroid(a, b, axis));

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

    /// Hands every primitive the ray might meet to `accumulator`.
    pub fn query(&self, ray: &Ray, near: f64, far: f64, accumulator: &mut dyn Accumulator<T>) -> () {
        self.traverse(ray, near, far, accumulator, &mut TraversalStats::default())
    }

    /// Same as [`BVHNode::query`], but adds the work done to `stats`.
    ///
    /// Both entry points run the very same `traverse`, so the measured traversal is the one the
    /// renderer performs — a separate instrumented copy would be free to drift away from it and
    /// would measure nothing trustworthy.
    pub fn query_instrumented(&self, ray: &Ray, near: f64, far: f64, accumulator: &mut dyn Accumulator<T>, stats: &mut TraversalStats) -> () {
        self.traverse(ray, near, far, accumulator, stats)
    }

    /// Number of primitives held across every leaf of this subtree.
    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
            + self.left.as_ref().map_or(0, |node| node.primitive_count())
            + self.right.as_ref().map_or(0, |node| node.primitive_count())
    }

    fn traverse(&self, ray: &Ray, near: f64, far: f64, accumulator: &mut dyn Accumulator<T>, stats: &mut TraversalStats) -> () {
        stats.box_tests += 1;
        if self.aabbox.hit(ray, near, far).is_none() {
            return;
        }
        stats.nodes_visited += 1;

        if self.primitives.len() > 0 {
            accumulator.accumulate(&mut self.primitives.clone())
        }
        else {
            Self::traverse_subnode(&self.left, ray, near, far, accumulator, stats);
            Self::traverse_subnode(&self.right, ray, near, far, accumulator, stats);
        }
    }

    fn traverse_subnode<'a>(
        node: &'a Option<Box<BVHNode<T>>>,
        ray: &Ray,
        near: f64,
        far: f64,
        accumulator: &mut dyn Accumulator<T>,
        stats: &mut TraversalStats,
    ) -> () {
        match &node {
            None => (),
            Some(node) => node.traverse(ray, near, far, accumulator, stats),
        }
    }

    /// The axis along which the primitives' centroids are most spread out.
    ///
    /// It replaces an axis drawn from an unseeded `random_double()`, and the gain is not one of
    /// taste. A random axis meant the tree changed shape from one run to the next, so **no
    /// traversal counter could be compared across two builds** — three consecutive runs of
    /// `cornell_box.stage` gave 9.63, 8.95 and 9.31 box tests per ray. An accelerator whose cost
    /// cannot be measured cannot be improved on purpose.
    ///
    /// The widest spread is also the better guess, not merely a reproducible one: it is the axis
    /// along which a plane separates the primitives most, and the one a full surface-area
    /// heuristic usually elects. Centroids rather than box corners, because a split assigns each
    /// primitive whole to one side and the centroid is what decides which — see
    /// `docs/heuristique_aire_surface.md` §4.
    fn widest_centroid_axis(primitives: &[T]) -> usize {
        let mut min = [f64::MAX; 3];
        let mut max = [f64::MIN; 3];

        for primitive in primitives.iter() {
            let centroid = primitive.get_bounding_box().centroid();
            for axis in 0..3 {
                min[axis] = min[axis].min(centroid[axis]);
                max[axis] = max[axis].max(centroid[axis]);
            }
        }

        let mut widest = 0;
        for axis in 1..3 {
            if max[axis] - min[axis] > max[widest] - min[widest] {
                widest = axis;
            }
        }

        widest
    }

    fn compare_centroid(a: &T, b: &T, axis: usize) -> Ordering {
        let a_pos = a.get_bounding_box().centroid()[axis];
        let b_pos = b.get_bounding_box().centroid()[axis];

        // `unwrap` is deliberate: `partial_cmp` only fails on `NaN`, which a bounded box cannot
        // produce, and the `debug_assert` in `new` is what keeps unbounded ones out. A silent
        // `Ordering::Equal` here would turn that into an arbitrary tree instead of a loud stop.
        a_pos.partial_cmp(&b_pos).unwrap()
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
