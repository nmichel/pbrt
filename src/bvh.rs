use std::cmp::Ordering;

use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::ray::Ray;

/// The root is the first node pushed by `BVH::new`, hence always at index 0.
const ROOT_NODE_IDX: usize = 0;

/// One node of a [`BVH`], holding indices rather than pointers.
///
/// `left_first` means two things depending on `primitive_count`, which is the compact encoding the
/// mesh BVH already uses: on a leaf it is where that leaf's primitives start in `BVH::primitives`,
/// on an interior node it is the index of its left child. The right child is always the next node
/// along, because children are pushed back to back — so an interior node needs one index, not two.
struct BVHNode {
    aabbox: AABoundingBox,
    left_first: usize,

    /// Primitives held by this node, and zero for an interior one. `is_leaf` is this test.
    primitive_count: usize,
}

impl BVHNode {
    fn is_leaf(&self) -> bool {
        self.primitive_count > 0
    }

    fn primitive_range(&self) -> std::ops::Range<usize> {
        self.left_first..(self.left_first + self.primitive_count)
    }
}

/// A bounding volume hierarchy over the scene's primitives, laid out flat.
///
/// The nodes live in one contiguous `Vec` and refer to each other by index; the primitives live in
/// a second one, permuted at build time so that every leaf owns a contiguous **range** of it. Two
/// allocations for the whole tree, where boxed children and a `Vec<T>` per leaf would cost one each
/// and scatter them wherever the allocator saw fit.
///
/// The point is not the allocations — a scene holds a handful of primitives, so they will not show
/// on a clock. It is that an index-addressed tree is what lets a traversal carry state per pending
/// node, which is what visiting the nearer child first and narrowing the search interval require.
/// `BVHTree` in `shapes::triangle_mesh` is the same layout, and does exactly that.
pub struct BVH<T>
where
    T: AABound,
{
    nodes: Vec<BVHNode>,

    /// Every primitive, permuted so that each leaf's are contiguous. The permutation *is* the
    /// partition: no primitive is copied into a node, and none is duplicated.
    primitives: Vec<T>,
}

/// What a traversal does with the primitives of a leaf it could not exclude.
pub trait Accumulator<T> {
    /// Receives a **borrowed slice**, never an owned vector: the primitives belong to the tree and
    /// a leaf is a range of it, so there is nothing to hand over but a view. What the accumulator
    /// then does with it is its own business.
    fn accumulate(&mut self, items: &[T]);
}

/// Work counters for traversals of the scene accelerator.
///
/// Same purpose as [`crate::shapes::triangle_mesh::TraversalStats`], and a separate type because
/// the work units differ — objects tested here, triangles there — and the two traversals are
/// different algorithms: the mesh's orders its children and narrows its interval, this one does
/// neither. A single type would abstract over that difference rather than describe it.
///
/// Reproducible to the unit for a given ray set: neither the build nor the traversal draws a random
/// number, so the same rays over the same scene give the same counts.
#[derive(Default, Clone, Copy)]
pub struct TraversalStats {
    /// Nodes whose contents were examined. A node whose box the ray misses is not counted — what
    /// it cost is a box test, which `box_tests` counts.
    pub nodes_visited: usize,

    /// Ray/box tests. Every node taken off the stack performs exactly one, on its own box, so this
    /// figure also counts the nodes the traversal reached at all.
    pub box_tests: usize,

    /// Objects handed to `Object::intersect`.
    ///
    /// Today this is also the number of primitives the accelerator hands out: it returns every
    /// candidate it cannot exclude, and the scene then tests all of them in whatever order they
    /// arrived. The two figures coincide only because nothing prunes between them — which is the
    /// defect, not a property.
    pub object_tests: usize,
}

impl<T: AABound> BVH<T> {
    /// Builds the tree over `primitives`, which it takes ownership of and permutes.
    ///
    /// # Precondition: `primitives` must not be empty
    ///
    /// A tree over nothing is not a thing this type can represent — its root would need a bounding
    /// box, and the empty box is not one a traversal may be given. Emptiness belongs one level up,
    /// in the `Option` that `Scene::build_bvh` returns, so it is asserted here rather than handled.
    /// Same reasoning as `AABoundingBox::hit`: "does a ray hit nothing" and "what does a tree over
    /// nothing look like" are questions with no useful answer.
    pub fn new(primitives: Vec<T>) -> Self {
        debug_assert!(
            !primitives.is_empty(),
            "BVH::new needs at least one primitive; an empty scene is a `None` tree, see Scene::build_bvh"
        );
        debug_assert!(
            primitives.iter().all(|primitive| primitive.get_bounding_box().is_bounded()),
            "BVH::new was given an unbounded primitive; those belong outside the accelerator, see Scene::commit"
        );

        let root = BVHNode {
            aabbox: AABoundingBox::empty(),
            left_first: 0,
            primitive_count: primitives.len(),
        };

        let mut tree = Self {
            nodes: vec![root],
            primitives,
        };
        tree.subdivide(ROOT_NODE_IDX);

        tree
    }

    /// Hands every primitive the ray might meet to `accumulator`.
    pub fn query(&self, ray: &Ray, near: f64, far: f64, accumulator: &mut dyn Accumulator<T>) -> () {
        self.traverse(ray, near, far, accumulator, &mut TraversalStats::default())
    }

    /// Same as [`BVH::query`], but adds the work done to `stats`.
    ///
    /// Both entry points run the very same `traverse`, so the measured traversal is the one the
    /// renderer performs — a separate instrumented copy would be free to drift away from it and
    /// would measure nothing trustworthy.
    pub fn query_instrumented(&self, ray: &Ray, near: f64, far: f64, accumulator: &mut dyn Accumulator<T>, stats: &mut TraversalStats) -> () {
        self.traverse(ray, near, far, accumulator, stats)
    }

    /// Number of primitives the tree was built over.
    pub fn primitive_count(&self) -> usize {
        self.primitives.len()
    }

    /// Splits the node at `node_idx` until each leaf holds a single primitive, and returns the
    /// bounds of the subtree it built.
    ///
    /// The bounds travel back up rather than being computed per node from its range, and that is
    /// deliberate: a node's box is the union of its two children's, so building it bottom-up costs
    /// exactly one `get_bounding_box()` per primitive, where computing it top-down at every level
    /// would cost one per primitive *per level*. That call is not cheap — for a `TriangleMesh` it
    /// scans every vertex — so the difference is not academic. See the `get_bounding_box` entry in
    /// `IDEAS.md`.
    fn subdivide(&mut self, node_idx: usize) -> AABoundingBox {
        let (first, count) = {
            let node = &self.nodes[node_idx];
            (node.left_first, node.primitive_count)
        };

        if count == 1 {
            let aabbox = self.primitives[first].get_bounding_box();
            self.nodes[node_idx].aabbox = aabbox;
            return aabbox;
        }

        // Order this node's own range by centroid, then halve it by count. Sorting the subrange in
        // place is what makes the two halves contiguous, which is what lets a leaf be a range
        // rather than a list.
        let axis = Self::widest_centroid_axis(&self.primitives[first..(first + count)]);
        self.primitives[first..(first + count)].sort_by(|a, b| Self::compare_centroid(a, b, axis));

        let left_count = count / 2;

        // Children back to back, so the parent only needs the index of the first.
        let left_node_idx = self.nodes.len();
        self.nodes.push(BVHNode {
            aabbox: AABoundingBox::empty(),
            left_first: first,
            primitive_count: left_count,
        });
        self.nodes.push(BVHNode {
            aabbox: AABoundingBox::empty(),
            left_first: first + left_count,
            primitive_count: count - left_count,
        });

        let node = &mut self.nodes[node_idx];
        node.left_first = left_node_idx;
        node.primitive_count = 0;

        let left_aabbox = self.subdivide(left_node_idx);
        let right_aabbox = self.subdivide(left_node_idx + 1);

        let aabbox = AABoundingBox::combine(&left_aabbox, &right_aabbox);
        self.nodes[node_idx].aabbox = aabbox;

        aabbox
    }

    fn traverse(&self, ray: &Ray, near: f64, far: f64, accumulator: &mut dyn Accumulator<T>, stats: &mut TraversalStats) -> () {
        let mut stack: Vec<usize> = Vec::with_capacity(64);
        stack.push(ROOT_NODE_IDX);

        while let Some(node_idx) = stack.pop() {
            let node = &self.nodes[node_idx];

            stats.box_tests += 1;
            if node.aabbox.hit(ray, near, far).is_none() {
                continue;
            }
            stats.nodes_visited += 1;

            if node.is_leaf() {
                accumulator.accumulate(&self.primitives[node.primitive_range()]);
            }
            else {
                // Right first, so the left child pops first. Nothing in this traversal depends on
                // the order — the accumulator collects every candidate and the scene ranks them
                // afterwards — but the order does decide which of two equidistant hits the scene
                // keeps, since it prefers the first it sees. Left to right is therefore a choice,
                // not an accident, however invisible.
                stack.push(node.left_first + 1);
                stack.push(node.left_first);
            }
        }
    }

    /// The axis along which the primitives' centroids are most spread out.
    ///
    /// Choosing it from the geometry rather than at random is what makes the build **reproducible**,
    /// and that is not a matter of taste: an axis drawn from an unseeded generator would give a tree
    /// of a different shape on every run, and **no traversal counter could then be compared across
    /// two builds**. An accelerator whose cost cannot be measured cannot be improved on purpose.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::vector3::Vector3f;

    /// The smallest `AABound` there is: a bounding box standing in for a primitive.
    ///
    /// The tree is generic and knows nothing about what it holds beyond its box — that is the
    /// whole point of the `AABound` seam — so nothing else is needed here. Testing with spheres
    /// and materials would only prove that the seam had been crossed.
    ///
    /// Note that it is not `Clone`, and does not need to be: the tree permutes the primitives it
    /// owns and never copies one, so `T: AABound` is the whole bound.
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

    fn spread_out(count: usize) -> Vec<Boxed> {
        (0..count).map(|i| unit_box_at(i as f64 * 3.0)).collect()
    }

    /// One primitive is a leaf, and the tree is that single node.
    #[test]
    fn test_single_primitive_is_a_leaf() {
        let tree = BVH::new(spread_out(1));

        assert_eq!(tree.nodes.len(), 1);
        assert!(tree.nodes[ROOT_NODE_IDX].is_leaf());
        assert_eq!(tree.nodes[ROOT_NODE_IDX].primitive_count, 1);
    }

    /// Building must terminate, and the root box must enclose every primitive.
    ///
    /// The termination half is not idle: the split axis follows the centroid spread, so the shape
    /// of the recursion depends on the geometry rather than on a constant.
    #[test]
    fn test_build_encloses_every_primitive() {
        let primitives = spread_out(7);

        let expected = primitives.iter().fold(AABoundingBox::empty(), |mut acc, primitive| {
            acc.combine_with(&primitive.get_bounding_box());
            acc
        });

        let tree = BVH::new(primitives);

        assert_eq!(tree.nodes[ROOT_NODE_IDX].aabbox.bmin, expected.bmin);
        assert_eq!(tree.nodes[ROOT_NODE_IDX].aabbox.bmax, expected.bmax);
    }

    /// The leaves partition the primitives: each one belongs to exactly one leaf.
    ///
    /// The leaves being ranges of a single vector is what makes this checkable at all: the ranges
    /// must tile it without gap or overlap. A leaf holding its own copy of its primitives could not
    /// state the invariant, since nothing would tie the copies back to one original.
    #[test]
    fn test_leaves_tile_the_primitives() {
        let tree = BVH::new(spread_out(7));

        let mut covered = vec![0usize; tree.primitive_count()];
        for node in tree.nodes.iter() {
            if node.is_leaf() {
                for i in node.primitive_range() {
                    covered[i] += 1;
                }
            }
        }

        assert!(
            covered.iter().all(|times| *times == 1),
            "every primitive must belong to exactly one leaf, got {:?}",
            covered
        );
    }

    /// An interior node's box encloses both of its children's.
    ///
    /// A conservative bound is the whole basis of the traversal: a ray rejected by a parent's box
    /// must be unable to reach anything below it. A parent smaller than a child would silently
    /// drop geometry.
    #[test]
    fn test_every_parent_encloses_its_children() {
        let tree = BVH::new(spread_out(9));

        for node in tree.nodes.iter() {
            if node.is_leaf() {
                continue;
            }

            for child_idx in [node.left_first, node.left_first + 1] {
                let child = &tree.nodes[child_idx];
                assert!(
                    node.aabbox.bmin.x <= child.aabbox.bmin.x
                        && node.aabbox.bmin.y <= child.aabbox.bmin.y
                        && node.aabbox.bmin.z <= child.aabbox.bmin.z
                        && node.aabbox.bmax.x >= child.aabbox.bmax.x
                        && node.aabbox.bmax.y >= child.aabbox.bmax.y
                        && node.aabbox.bmax.z >= child.aabbox.bmax.z,
                    "a parent must enclose its child: {:?} does not contain {:?}",
                    node.aabbox,
                    child.aabbox
                );
            }
        }
    }
}
