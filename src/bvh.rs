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

/// One pending node of an ordered traversal, and the distance at which the ray enters its box.
///
/// Carrying the distance is what allows **one box test per node**: the test that decided the
/// visiting order already computed it, so the pop does not have to test the box again to recover
/// it. It is still worth re-examining at pop, against the narrowed interval — a hit found since the
/// push may sit closer than this whole node.
struct StackEntry {
    node_idx: usize,
    entry_distance: f64,
}

/// Work counters for traversals of the scene accelerator.
///
/// Same purpose as [`crate::shapes::triangle_mesh::TraversalStats`], and a separate type because
/// the work units differ — objects tested here, triangles there. A single type would abstract over
/// that difference rather than describe it.
///
/// Reproducible to the unit for a given ray set: neither the build nor the traversal draws a random
/// number, so the same rays over the same scene give the same counts.
#[derive(Default, Clone, Copy)]
pub struct TraversalStats {
    /// Nodes whose contents were examined.
    ///
    /// Two kinds of node are deliberately absent. One whose box the ray misses never reaches the
    /// stack, and one popped after the interval has shrunk past its entry distance is dropped
    /// without a single test — no work was done on either. What the first cost is a box test, which
    /// `box_tests` counts; the second cost nothing at all.
    pub nodes_visited: usize,

    /// Ray/box tests, every one of them routed through `BVH::hit_box` so this cannot drift.
    ///
    /// In the ordered traversal a node's box is tested once, by the parent that decides whether and
    /// in what order to push it, plus one test for the root, which has no parent. The unordered
    /// traversal of `query_p` tests instead at pop, one per node it takes off the stack.
    pub box_tests: usize,

    /// Primitives handed to the visitor, which is to say offered for a real geometric test.
    ///
    /// Fewer than the primitives the ray's path crosses, and that gap is the point: the interval
    /// shrinks to each hit as it is reported, so a subtree beginning beyond the nearest hit so far
    /// is never opened and its primitives are never offered.
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

    /// Offers `visit` the primitives the ray might meet, nearest node first, narrowing the search as
    /// `visit` reports hits. Adds the work done to `stats`.
    ///
    /// # The contract with `visit`
    ///
    /// Called as `visit(primitive, near, far)`, it must answer with the distance of the hit it
    /// **adopted as its new nearest**, or `None` when it kept nothing. The interval it receives is
    /// the one still worth searching: `far` shrinks to the last adopted distance, so whatever the
    /// visitor is offered next lies closer than everything it has already kept.
    ///
    /// That contract is the whole reason this takes a closure rather than filling a list. A list is
    /// complete before the first primitive is tested, so nothing in it can be excluded by what a
    /// test found: every candidate is tested over the caller's full `[near, far]`, and the ranking
    /// happens afterwards, once the work is already spent. Reporting each hit as it is found lets
    /// the traversal skip subtrees that begin beyond it, and lets each primitive reject over a
    /// shorter interval than the one before.
    ///
    /// The visitor is free to decline a hit it was offered — returning `None` leaves the interval
    /// untouched — which is what lets the caller keep its own tie-breaking rule.
    pub fn query<F>(&self, ray: &Ray, near: f64, far: f64, mut visit: F, stats: &mut TraversalStats)
    where
        F: FnMut(&T, f64, f64) -> Option<f64>,
    {
        // The caller's `far`, narrowed by every hit reported so far. It bounds the box tests as well
        // as the primitive tests: a node the ray enters beyond the nearest hit cannot hold a nearer
        // one, so it is not worth opening.
        let mut min_t = far;

        // The root has no parent to have tested it, so it is tested here. That is the one box test
        // outside the loop, and the reason a ray missing the scene entirely costs exactly one.
        let root_distance = match Self::hit_box(&self.nodes[ROOT_NODE_IDX], ray, near, min_t, stats) {
            Some(distance) => distance,
            None => return,
        };

        let mut stack: Vec<StackEntry> = Vec::with_capacity(64);
        stack.push(StackEntry {
            node_idx: ROOT_NODE_IDX,
            entry_distance: root_distance,
        });

        while let Some(entry) = stack.pop() {
            // A hit adopted since this node was pushed may sit closer than the point where the ray
            // enters it, in which case nothing below it can contribute and it is dropped without a
            // single test. This is what visiting the nearer child first buys: it gives `min_t` its
            // best chance to shrink before the farther child is ever popped.
            if entry.entry_distance >= min_t {
                continue;
            }

            stats.nodes_visited += 1;
            let node = &self.nodes[entry.node_idx];

            if node.is_leaf() {
                for primitive in self.primitives[node.primitive_range()].iter() {
                    stats.object_tests += 1;
                    if let Some(distance) = visit(primitive, near, min_t) {
                        debug_assert!(
                            distance >= near && distance <= min_t,
                            "the visitor adopted a hit at {} but was offered [{}, {}]; a hit outside the interval would let the traversal prune \
                             geometry that is actually nearer",
                            distance,
                            near,
                            min_t
                        );
                        min_t = distance;
                    }
                }
            }
            else {
                let left_idx = node.left_first;
                let right_idx = left_idx + 1;
                let left = (left_idx, Self::hit_box(&self.nodes[left_idx], ray, near, min_t, stats));
                let right = (right_idx, Self::hit_box(&self.nodes[right_idx], ray, near, min_t, stats));

                // Order the two children so the nearer is examined first. Only their entry distances
                // are compared: a child the ray misses is never pushed (see code below), so where it would have sat
                // is not relevant.
                let (nearer, farther) = match (left.1, right.1) {
                    (Some(left_distance), Some(right_distance)) if right_distance < left_distance => (right, left),
                    _ => (left, right),
                };

                // The stack is LIFO, so pushing the farther child first pops the nearer first.
                // Children that the ray misses (None) are not pushed,
                // so the visitor is never offered a primitive it cannot meet.
                for (child_idx, child_hit) in [farther, nearer] {
                    if let Some(entry_distance) = child_hit {
                        stack.push(StackEntry {
                            node_idx: child_idx,
                            entry_distance,
                        });
                    }
                }
            }
        }
    }

    /// Whether any primitive the ray might meet satisfies `blocks`. Adds the work done to `stats`.
    ///
    /// Deliberately **not** the ordered traversal. Ordering and narrowing both exist to reach the
    /// *nearest* hit sooner, and here any hit settles the question: there is no interval to shrink,
    /// nothing for an early hit to prune, so the children are pushed as they come and the stack
    /// holds plain indices instead of entry distances. What this traversal does that the other
    /// cannot is stop — it returns on the first `blocks`.
    pub fn query_p<F>(&self, ray: &Ray, near: f64, far: f64, mut blocks: F, stats: &mut TraversalStats) -> bool
    where
        F: FnMut(&T) -> bool,
    {
        let mut stack: Vec<usize> = Vec::with_capacity(64);
        stack.push(ROOT_NODE_IDX);

        while let Some(node_idx) = stack.pop() {
            let node = &self.nodes[node_idx];

            if Self::hit_box(node, ray, near, far, stats).is_none() {
                continue;
            }
            stats.nodes_visited += 1;

            if node.is_leaf() {
                for primitive in self.primitives[node.primitive_range()].iter() {
                    stats.object_tests += 1;
                    if blocks(primitive) {
                        return true;
                    }
                }
            }
            else {
                // Right pushed first, so the left child pops first. No hit is preferred over
                // another here, but the order still decides *how soon* the first occluder is met,
                // and the primitives of a leaf sit in build order: walking them left to right is
                // what makes the search stop early on the near side of the scene rather than the
                // far one. Reversing these two lines takes `cornell_box.stage`'s shadow rays from
                // 0.89 object tests per ray to 0.92.
                stack.push(node.left_first + 1);
                stack.push(node.left_first);
            }
        }

        false
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

    /// Tests `node`'s box against `ray` and records the test.
    ///
    /// Every ray/box test of both traversals goes through here, so `box_tests` cannot drift out of
    /// step with the tests actually performed.
    fn hit_box(node: &BVHNode, ray: &Ray, near: f64, far: f64, stats: &mut TraversalStats) -> Option<f64> {
        stats.box_tests += 1;
        node.aabbox.hit(ray, near, far)
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

    /// Unit boxes at `x = 0, 0.5, 1.0, …`, so that they overlap and a ray along +x meets them all.
    ///
    /// The overlap is the point: with `spread_out` the first hit prunes everything behind it, so a
    /// traversal never gets to offer a second primitive. Overlapping boxes are what let a test watch
    /// the interval shrink across several calls.
    fn stacked_up(count: usize) -> Vec<Boxed> {
        (0..count).map(|i| unit_box_at(i as f64 * 0.5)).collect()
    }

    /// A ray along +x, starting one unit before the first box of either helper.
    fn along_x() -> Ray {
        Ray::new(&Vector3f::new(-1.0, 0.5, 0.5), &Vector3f::new(1.0, 0.0, 0.0))
    }

    const NEAR: f64 = 0.0001;
    const FAR: f64 = 1000.0;

    /// A visitor that adopts nothing must still be offered every primitive the ray meets.
    ///
    /// This is the invariant the whole change rests on, and the one whose failure is silent: an
    /// over-eager prune drops geometry without any error, and the image simply loses an object.
    /// Adopting nothing keeps the interval at the caller's `far`, so nothing may be excluded.
    #[test]
    fn test_a_visitor_that_adopts_nothing_is_offered_every_primitive() {
        let tree = BVH::new(stacked_up(4));

        let mut offered = 0;
        tree.query(
            &along_x(),
            NEAR,
            FAR,
            |_primitive, _near, _far| {
                offered += 1;
                None
            },
            &mut TraversalStats::default(),
        );

        assert_eq!(offered, 4, "all four boxes lie along the ray and none was excluded by a hit");
    }

    /// The interval handed to the visitor shrinks to the last hit it adopted.
    ///
    /// The visitor adopts once, at a distance chosen beyond every box's entry point so that nothing
    /// is pruned and the later calls can be observed. What they must show is the narrowed `far`:
    /// without it every primitive is tested over the caller's whole interval, which is precisely
    /// what filling a list of candidates forced.
    #[test]
    fn test_the_interval_shrinks_to_the_adopted_hit() {
        let tree = BVH::new(stacked_up(4));
        let adopted_at = 5.0;

        let mut intervals: Vec<f64> = Vec::new();
        tree.query(
            &along_x(),
            NEAR,
            FAR,
            |_primitive, _near, far| {
                intervals.push(far);
                if intervals.len() == 1 {
                    Some(adopted_at)
                }
                else {
                    None
                }
            },
            &mut TraversalStats::default(),
        );

        assert_eq!(intervals[0], FAR, "the first call has nothing to narrow it");
        assert!(intervals.len() > 1, "the remaining boxes are entered before {}", adopted_at);
        assert!(
            intervals[1..].iter().all(|far| *far == adopted_at),
            "every later call must be bounded by the adopted hit, got {:?}",
            intervals
        );
    }

    /// One hit near the ray's origin is enough to prune the whole rest of the tree.
    ///
    /// Ordering and narrowing only pay together: narrowing needs a hit to narrow *with*, and the
    /// nearest hit comes first only if the nearer child is opened first. Separated boxes and a
    /// visitor that adopts whatever box it is handed make both visible — the count must be 1, not 8.
    ///
    /// **Both directions along the axis, and that is what makes this about ordering.** The build
    /// sorts each range by centroid, so for a ray towards +x the left child is always the nearer
    /// one and pushing the children in build order happens to be right. Only the ray running back
    /// down the axis tells the two apart: there the far half comes first in build order, gets opened
    /// first, and its hit is adopted before the near half is ever reached.
    #[test]
    fn test_the_nearest_hit_prunes_the_rest_of_the_tree() {
        // 8 disjoint unit boxes, 3 units apart, so a ray along +x meets them all in order.
        let tree = BVH::new(spread_out(8));

        // The boxes span x ∈ [0, 22], so both origins sit outside and look across the whole layout.
        for (origin_x, direction_x) in [(-1.0, 1.0), (25.0, -1.0)] {
            let ray = Ray::new(&Vector3f::new(origin_x, 0.5, 0.5), &Vector3f::new(direction_x, 0.0, 0.0));
            let mut stats = TraversalStats::default();

            let mut offered = 0;
            tree.query(
                &ray,
                NEAR,
                FAR,
                |primitive, near, far| {
                    offered += 1;
                    primitive.0.hit(&ray, near, far)
                },
                &mut stats,
            );

            assert_eq!(
                offered, 1,
                "towards x{:+}, the nearest box is met first and no other can then be nearer",
                direction_x
            );
            assert_eq!(stats.object_tests, 1, "the counter must agree with the calls actually made");
        }
    }

    /// `query_p` finds a blocker exactly when `query` is offered a primitive the ray meets.
    ///
    /// The two are separate traversals — one ordered and narrowing, the other unordered with an
    /// early return — so nothing but a test keeps them answering the same question. The failure they
    /// would hide is quiet: a `query_p` that missed occluders would neither crash nor slow anything
    /// down, it would only delete shadows.
    #[test]
    fn test_query_p_agrees_with_query() {
        // 8 disjoint unit boxes, 3 units apart
        let tree = BVH::new(spread_out(8));

        // Rays across the whole layout: some through the boxes, some between them, some past the
        // end, and one pointing away.
        let origins = [-1.0, 4.0, 10.0, 25.0];
        let heights = [0.5, 1.5];
        let directions = [1.0, -1.0];

        for x in origins {
            for y in heights {
                for direction in directions {
                    let ray = Ray::new(&Vector3f::new(x, y, 0.5), &Vector3f::new(direction, 0.0, 0.0));

                    let mut met_any = false;
                    tree.query(
                        &ray,
                        NEAR,
                        FAR,
                        |primitive, near, far| {
                            let hit = primitive.0.hit(&ray, near, far);
                            met_any |= hit.is_some();
                            hit
                        },
                        &mut TraversalStats::default(),
                    );

                    let blocked = tree.query_p(
                        &ray,
                        NEAR,
                        FAR,
                        |primitive| primitive.0.hit(&ray, NEAR, FAR).is_some(),
                        &mut TraversalStats::default(),
                    );

                    assert_eq!(
                        blocked, met_any,
                        "the two traversals disagree for a ray from ({}, {}) towards x{:+}",
                        x, y, direction
                    );
                }
            }
        }
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
