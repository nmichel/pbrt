use super::triangle_mesh::{intersect_ray, TriangleIntersection};
use crate::geom::aabound::AABoundingBox;
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use std::sync::Arc;

/// Work counters for traversals of the tree.
///
/// The quality of a BVH is invisible in the rendered image: a poor split plane produces
/// exactly the same picture, only more slowly. These counters are therefore the only
/// observable against which a change to the split heuristic can be judged.
///
/// They are exactly reproducible for a given set of rays — a traversal draws no random
/// numbers — which is why tree quality can be measured today, while the sampler is still
/// unseeded and every lighting result is still statistical.
///
/// Read them as work *per ray*: accumulate over a ray set, then divide by its size.
#[derive(Default, Clone, Copy)]
pub struct TraversalStats {
    /// Nodes whose contents were examined — triangles tested, or the two children's boxes
    /// tested.
    ///
    /// A node taken off the stack and immediately dropped because `min_t` has shrunk past its
    /// entry distance is *not* counted: no work was done on it. Nor is one whose box the ray
    /// misses, since it never reaches the stack. What those cost is a box test, and
    /// `box_tests` counts that.
    pub nodes_visited: usize,

    /// Ray/box tests, the dominant cost of an interior node.
    ///
    /// Counted apart from `nodes_visited` because the two do not move together. An *interior*
    /// node examined performs two box tests, one per child; a leaf performs none, testing
    /// triangles instead; a node discarded at pop performs none either. So
    ///
    /// ```text
    /// box_tests = 2 · (interior nodes examined) + 1
    /// ```
    ///
    /// the `+ 1` being the root, which has no parent to test it. The two counters together
    /// therefore split a traversal into its interior and leaf halves, which is more than
    /// either says alone.
    pub box_tests: usize,

    /// Ray/triangle tests, i.e. calls to `intersect_ray`. This is the figure a better
    /// tree is meant to bring down: it counts the primitives the tree failed to exclude.
    pub triangle_tests: usize,
}

/// Exact description of the tree the build produced.
///
/// The counterpart of [`TraversalStats`], and read differently: this is a complete reading
/// of a single tree, taken once after `build`, whereas traversal counters are summed over a
/// ray set and reported per ray. Nothing here is averaged or accumulated — hence the
/// prefix, which names the moment of the BVH's life being described rather than the
/// quantity.
///
/// [`TraversalStats`] says how much work a ray did; this says why it did it.
///
/// A tree that is deep but has few triangles in its leaves is not the same as one that is
/// shallow but has many triangles in its leaves.
///
/// A heuristic that lowers the traversal counters while doubling the depth, or
/// that leaves 200-triangle leaves behind, is not the same improvement as one that does
/// neither — and only these figures tell the two apart.
#[derive(Default, Clone, Copy)]
pub struct BuildStats {
    pub node_count: usize,
    pub leaf_count: usize,

    /// Depth of the deepest leaf, root counted as depth 1.
    pub max_depth: usize,

    /// Triangles in the largest leaf: the worst case a single ray can be charged for.
    pub max_leaf_tri_count: usize,

    /// Triangles summed over all leaves. Every triangle belongs to exactly one leaf, so
    /// this must equal the mesh's triangle count — a partition that loses or duplicates a
    /// triangle shows up here and nowhere else.
    pub total_leaf_tri_count: usize,
}

/// The root is the first node pushed by `build`, hence always at index 0.
const ROOT_NODE_IDX: usize = 0;

struct BVHNode {
    bbox: AABoundingBox,
    left_first: usize,
    tri_count: usize,
}

/// A node still to visit, and the distance at which the ray enters its box.
///
/// Carrying the distance is what lets each box be tested once: it is measured when the parent
/// looks at its children — which it must do anyway, to order them — and travels here instead
/// of being recomputed at pop. See `BVHTree::traverse`.
#[derive(Copy, Clone)]
struct StackEntry {
    node_idx: usize,
    entry_distance: f64,
}

impl BVHNode {
    pub fn is_leaf(&self) -> bool {
        self.tri_count > 0
    }
}

#[derive(Clone, Copy)]
struct Tri {
    vertex0_idx: usize,
    vertex1_idx: usize,
    vertex2_idx: usize,
    centroid: Vector3f,
}

pub struct BVHTree {
    tris: Vec<Tri>,
    tri_idx: Vec<usize>,
    nodes: Vec<BVHNode>,
    vertices: Arc<Vec<f64>>,
}

/// Number of bins the centroid extent of a node is divided into, per axis.
///
/// Binning is what brings the build down from O(N²) to O(N + K) per node: one pass fills the
/// bins, a second scans their internal boundaries. The count is the quality/build-time knob —
/// more bins approach the exhaustive heuristic and cost more to build. pbrt uses 12.
/// See `docs/heuristique_aire_surface.md` §4.
const BIN_COUNT: usize = 8;

/// Candidate split planes per axis: the boundaries *between* consecutive bins, so one fewer
/// than there are bins. Boundary `i` separates bins `0..=i` from bins `i+1..`, per `[6]`.
const SPLIT_COUNT: usize = BIN_COUNT - 1;

#[derive(Copy, Clone)]
struct Bin {
    bounds: AABoundingBox,
    tri_count: usize,
}

/// A candidate split plane, identified by the bin boundary it sits on.
///
/// The plane is deliberately **not** carried as a position. `subdivide` has to partition the
/// triangles into exactly the two groups whose counts produced `cost`, and rebuilding a float
/// position from the boundary index would be a second, differently-rounded computation that
/// can disagree with the binning for a centroid sitting on a boundary. Carrying the binning
/// parameters instead lets the partition call the very same `bin_index`, so prediction and
/// partition coincide by construction. See `docs/heuristique_aire_surface.md` §4,
/// « Partitionner par indice de bin ».
struct SplitCandidate {
    axis: usize,

    /// Index of the boundary; the left side is bins `0..=boundary`. `[6]`
    boundary: usize,

    /// The two parameters `bin_index` needs: lower end of the centroid extent along `axis`,
    /// and the width of one bin.
    centroid_min: f64,
    bin_width: f64,

    /// `A_L·N_L + A_R·N_R` of `[3]`, un-normalised so it compares directly against
    /// `calculate_node_cost` — see `[5]`.
    cost: f64,
}

/// Which bin a centroid coordinate falls into. `[6]`
///
/// The single source of truth for that mapping: the cost model bins with it and the partition
/// classifies with it, so the two cannot disagree. See `docs/heuristique_aire_surface.md` §4.
///
/// The upper clamp catches `centroid_pos == centroid_max`, whose quotient is exactly
/// `BIN_COUNT`, along with any rounding that pushes the quotient a hair past the last bin.
fn bin_index(centroid_pos: f64, centroid_min: f64, bin_width: f64) -> usize {
    let quotient = (centroid_pos - centroid_min) / bin_width;
    (quotient.floor() as usize).clamp(0, BIN_COUNT - 1)
}

impl BVHTree {
    pub fn new(vertices: Arc<Vec<f64>>, indices: &Vec<usize>) -> Self {
        assert!(indices.len() % 3 == 0);

        let verts = vertices.as_ref();

        let triangle_count = indices.len() / 3;
        let mut tris = Vec::with_capacity(triangle_count);
        let mut tri_idx = Vec::with_capacity(triangle_count);
        let nodes = Vec::with_capacity(triangle_count * 2);

        let mut base = 0;
        for i in 0..triangle_count {
            let vertex0_idx = indices[base] as usize;
            let vertex1_idx = indices[base + 1] as usize;
            let vertex2_idx = indices[base + 2] as usize;

            let vertex0 = BVHTree::build_vertex(verts, vertex0_idx);
            let vertex1 = BVHTree::build_vertex(verts, vertex1_idx);
            let vertex2 = BVHTree::build_vertex(verts, vertex2_idx);

            let centroid = BVHTree::tri_centroid(&vertex0, &vertex1, &vertex2);

            tris.push(Tri {
                vertex0_idx,
                vertex1_idx,
                vertex2_idx,
                centroid,
            });

            tri_idx.push(i);

            base = base + 3;
        }

        BVHTree {
            tris,
            tri_idx,
            nodes,
            vertices,
        }
    }

    pub fn build(&mut self) {
        let mut root = BVHNode {
            bbox: AABoundingBox::empty(),
            left_first: 0,
            tri_count: self.tris.len(),
        };
        self.update_bounds(&mut root);
        self.nodes.push(root);
        self.subdivide(0);
    }

    fn update_bounds(&self, node: &mut BVHNode) {
        for i in node.left_first..(node.left_first + node.tri_count) {
            let tri = &self.tris[self.tri_idx[i]];
            let aabound = self.aabound_from_triangle(tri.vertex0_idx, tri.vertex1_idx, tri.vertex2_idx);
            node.bbox.combine_with(&aabound);
        }
    }

    fn subdivide(&mut self, node_idx: usize) {
        // Read the node's data out rather than holding a reference: the `nodes` vector is
        // pushed to below, which may move it in memory.
        let (left_first, tri_count) = {
            let node = &self.nodes[node_idx];
            (node.left_first, node.tri_count)
        };

        // `[5]`: splitting is worth it only when it beats keeping the node as a leaf. Both
        // sides of the comparison are un-normalised — see `calculate_node_cost`.
        let leaf_cost = self.calculate_node_cost(&self.nodes[node_idx]);
        let split = match self.find_best_split_plane(&self.nodes[node_idx]) {
            Some(split) if split.cost < leaf_cost => split,
            _ => return,
        };

        // In-place partition, classifying with the very same `bin_index` the cost model
        // binned with, so the two groups are exactly those whose counts produced
        // `split.cost`. Single forward pass: every triangle belonging left is swapped to the
        // front, which needs no backward index and so cannot underflow.
        let mut left_end = left_first;
        for i in left_first..(left_first + tri_count) {
            let centroid_pos = self.tris[self.tri_idx[i]].centroid[split.axis];
            if bin_index(centroid_pos, split.centroid_min, split.bin_width) <= split.boundary {
                self.tri_idx.swap(i, left_end);
                left_end += 1;
            }
        }
        let left_count = left_end - left_first;

        if left_count == 0 || left_count == tri_count {
            // Unreachable: `find_best_split_plane` only returns candidates with both sides
            // non-empty, and the loop above classifies with the same pure function over the
            // same triangles, so the counts must agree. Kept as a guard all the same, because
            // the alternative is an empty child and unbounded recursion.
            debug_assert!(
                false,
                "partition disagreed with the binned counts: {} of {} triangles on the left",
                left_count, tri_count
            );
            return;
        }

        let mut left_node = BVHNode {
            bbox: AABoundingBox::empty(),
            left_first: left_first,
            tri_count: left_count,
        };
        self.update_bounds(&mut left_node);

        let mut right_node = BVHNode {
            bbox: AABoundingBox::empty(),
            left_first: left_end,
            tri_count: tri_count - left_count,
        };
        self.update_bounds(&mut right_node);

        // The two children are pushed back to back, so the parent only needs the index of
        // the first: `left_first` doubles as "index of the left child" on an interior node.
        let left_node_idx = self.nodes.len();
        self.nodes.push(left_node);
        self.nodes.push(right_node);

        let node = &mut self.nodes[node_idx];
        node.left_first = left_node_idx;
        node.tri_count = 0;

        self.subdivide(left_node_idx);
        self.subdivide(left_node_idx + 1);
    }

    /// `A_L·N_L + A_R·N_R` for `split`, obtained by scanning every triangle of `node`.
    ///
    /// The reference implementation of `[3]`: no bins, the two child boxes accumulated
    /// directly. It is not used by the build — it is the oracle the binned prefix/suffix scan
    /// is checked against, and that check is what would have caught the per-bin area defect
    /// this replaces (`docs/heuristique_aire_surface.md` §3).
    ///
    /// It classifies with `bin_index`, exactly as the binned path does, so the two compare the
    /// same partition; the test then bears on the accumulation alone, which is what it claims
    /// to verify.
    ///
    /// Note that a cost of `0.0` is a perfectly legitimate result — a flat box, or a node
    /// whose triangles are all coplanar with the split — and is returned as such.
    #[cfg(test)]
    fn exhaustive_split_cost(&self, node: &BVHNode, split: &SplitCandidate) -> f64 {
        let mut left_box = AABoundingBox::empty();
        let mut right_box = AABoundingBox::empty();
        let mut left_count: usize = 0;
        let mut right_count: usize = 0;

        for i in node.left_first..(node.left_first + node.tri_count) {
            let tri = &self.tris[self.tri_idx[i]];
            let bounds = self.aabound_from_triangle(tri.vertex0_idx, tri.vertex1_idx, tri.vertex2_idx);
            let centroid_pos = tri.centroid[split.axis];

            if bin_index(centroid_pos, split.centroid_min, split.bin_width) <= split.boundary {
                left_count += 1;
                left_box.combine_with(&bounds);
            }
            else {
                right_count += 1;
                right_box.combine_with(&bounds);
            }
        }

        left_box.half_area() * left_count as f64 + right_box.half_area() * right_count as f64
    }

    /// Nearest intersection of `ray` with the mesh, within `[near, far]`.
    pub fn query(&self, ray: &Ray, near: f64, far: f64) -> Option<(TriangleIntersection, usize)> {
        self.traverse(ray, near, far, &mut TraversalStats::default())
    }

    /// Same as [`BVHTree::query`], but adds the work done to `stats`.
    ///
    /// Both entry points run the very same `traverse`, so the measured traversal is the
    /// one the renderer performs — a separate instrumented copy would be free to drift
    /// away from it and would measure nothing trustworthy.
    pub fn query_instrumented(&self, ray: &Ray, near: f64, far: f64, stats: &mut TraversalStats) -> Option<(TriangleIntersection, usize)> {
        self.traverse(ray, near, far, stats)
    }

    /// Whether `ray` meets any triangle of the mesh within `[near, far]`.
    ///
    /// The any-hit counterpart of [`BVHTree::query`], and deliberately a separate traversal rather
    /// than a flag on that one, because almost everything it does is unnecessary here:
    ///
    /// - **no ordering.** Visiting the nearer child first exists to shrink `min_t` early; with no
    ///   `min_t` to shrink there is nothing to gain, so the children are pushed as they come and
    ///   the stack holds plain indices instead of entry distances.
    /// - **no interval narrowing**, for the same reason. `[near, far]` is fixed throughout.
    /// - **no bookkeeping.** The first triangle found ends the traversal, so there is no nearest
    ///   to keep, no index to remember, and no comparison per hit.
    ///
    /// The shading quantities are not skipped here but one level up, in `TriangleMesh`: this
    /// returns a boolean, so no caller can ask for the uv interpolation or the ∂p/∂u and ∂p/∂v
    /// that `Intersectable::intersect` computes.
    ///
    /// Not instrumented. `TraversalStats` is threaded through `traverse` because something counts
    /// it; nothing counts this path yet, and an unused parameter is noise rather than symmetry.
    pub fn intersect_p(&self, ray: &Ray, near: f64, far: f64) -> bool {
        let verts = self.vertices.as_ref();

        // As in `traverse`: the root has no parent to test it, every other box is tested once, by
        // the parent that pushes it.
        if self.nodes[ROOT_NODE_IDX].bbox.hit(ray, near, far).is_none() {
            return false;
        }

        let mut stack: Vec<usize> = Vec::with_capacity(64);
        stack.push(ROOT_NODE_IDX);

        while let Some(node_idx) = stack.pop() {
            let node = &self.nodes[node_idx];

            if node.is_leaf() {
                for i in 0..node.tri_count {
                    let tri: &Tri = &self.tris[self.tri_idx[node.left_first + i]];
                    let p0 = BVHTree::build_vertex(verts, tri.vertex0_idx);
                    let p1 = BVHTree::build_vertex(verts, tri.vertex1_idx);
                    let p2 = BVHTree::build_vertex(verts, tri.vertex2_idx);

                    if let Some(intersection) = intersect_ray(ray, &p0, &p1, &p2) {
                        if intersection.t >= near && intersection.t <= far {
                            return true;
                        }
                    }
                }
            }
            else {
                for child_idx in [node.left_first, node.left_first + 1] {
                    if self.nodes[child_idx].bbox.hit(ray, near, far).is_some() {
                        stack.push(child_idx);
                    }
                }
            }
        }

        false
    }

    /// Tests `node`'s bounding box against `ray` and records the test.
    ///
    /// Every ray/box test in the traversal goes through here, so `box_tests` cannot drift
    /// out of step with the tests actually performed.
    fn hit_box(node: &BVHNode, ray: &Ray, near: f64, far: f64, stats: &mut TraversalStats) -> Option<f64> {
        stats.box_tests += 1;
        node.bbox.hit(ray, near, far)
    }

    /// Ordered depth-first traversal, nearest child first.
    ///
    /// **Each node's box is tested exactly once.** The distance at which the ray enters a box
    /// is what decides the visiting order, so it has to be known before a child is pushed;
    /// carrying it on the stack means the pop does not have to test the box again to recover
    /// it. Re-testing was the previous shape of this loop, and it cost one redundant test per
    /// node — about 60 % of all box tests.
    ///
    /// The distance is still worth re-examining at pop, but only against `min_t`: a hit found
    /// since the push may have brought the nearest intersection closer than this whole node,
    /// in which case its subtree cannot contribute and is skipped without any test at all.
    /// That is the entire point of visiting the near child first — it gives `min_t` the best
    /// chance of shrinking before the far child is considered.
    fn traverse(&self, ray: &Ray, near: f64, far: f64, stats: &mut TraversalStats) -> Option<(TriangleIntersection, usize)> {
        let verts = self.vertices.as_ref();
        let mut min_t: f64 = f64::MAX;
        let mut current_intersection: Option<TriangleIntersection> = None;
        let mut current_tri_idx: usize = 0;

        // The root has no parent to test it, so it is tested here — the one box test outside
        // the loop, and the reason a ray that misses the mesh entirely costs exactly one.
        let root_distance = match BVHTree::hit_box(&self.nodes[ROOT_NODE_IDX], ray, near, far, stats) {
            Some(distance) => distance,
            None => return None,
        };

        let mut stack: Vec<StackEntry> = Vec::with_capacity(64);
        stack.push(StackEntry {
            node_idx: ROOT_NODE_IDX,
            entry_distance: root_distance,
        });

        while let Some(entry) = stack.pop() {
            if entry.entry_distance >= min_t {
                continue;
            }

            stats.nodes_visited += 1;
            let node = &self.nodes[entry.node_idx];

            if node.is_leaf() {
                for i in 0..node.tri_count {
                    let tri_idx = node.left_first + i;
                    let tri: &Tri = &self.tris[self.tri_idx[tri_idx]];
                    let p0 = BVHTree::build_vertex(verts, tri.vertex0_idx);
                    let p1 = BVHTree::build_vertex(verts, tri.vertex1_idx);
                    let p2 = BVHTree::build_vertex(verts, tri.vertex2_idx);
                    stats.triangle_tests += 1;
                    if let Some(intersection) = intersect_ray(ray, &p0, &p1, &p2) {
                        if intersection.t < min_t && intersection.t >= near && intersection.t <= far {
                            min_t = intersection.t;
                            current_intersection.replace(intersection);
                            current_tri_idx = self.tri_idx[tri_idx];
                        }
                    }
                }
            }
            else {
                let left_idx = node.left_first;
                let right_idx = node.left_first + 1;
                let left = (left_idx, BVHTree::hit_box(&self.nodes[left_idx], ray, near, far, stats));
                let right = (right_idx, BVHTree::hit_box(&self.nodes[right_idx], ray, near, far, stats));

                // Order the two children so the nearer is examined first. Only their entry
                // distances are compared: a child the ray misses at all never gets pushed, so
                // its position in the order is irrelevant.
                let (nearer, farther) = match (left.1, right.1) {
                    (Some(left_distance), Some(right_distance)) if right_distance < left_distance => (right, left),
                    _ => (left, right),
                };

                // The stack is LIFO, so pushing the farther child first pops the nearer first.
                for (child_idx, child_hit) in [farther, nearer] {
                    if let Some(entry_distance) = child_hit {
                        if entry_distance < min_t {
                            stack.push(StackEntry {
                                node_idx: child_idx,
                                entry_distance,
                            });
                        }
                    }
                }
            }
        }

        match current_intersection {
            Some(intersection) => Some((intersection, current_tri_idx)),
            _ => return None,
        }
    }

    /// Number of triangles the tree was built over.
    pub fn triangle_count(&self) -> usize {
        self.tris.len()
    }

    /// Walks the finished tree and reports its shape.
    pub fn build_stats(&self) -> BuildStats {
        let mut stats = BuildStats::default();
        let root_depth = 1;
        self.collect_build_stats(0, root_depth, &mut stats);
        stats
    }

    fn collect_build_stats(&self, node_idx: usize, depth: usize, stats: &mut BuildStats) {
        let node = &self.nodes[node_idx];
        stats.node_count += 1;
        stats.max_depth = stats.max_depth.max(depth);

        if node.is_leaf() {
            stats.leaf_count += 1;
            stats.total_leaf_tri_count += node.tri_count;
            stats.max_leaf_tri_count = stats.max_leaf_tri_count.max(node.tri_count);
        }
        else {
            self.collect_build_stats(node.left_first, depth + 1, stats);
            self.collect_build_stats(node.left_first + 1, depth + 1, stats);
        }
    }

    fn aabound_from_triangle(&self, i0: usize, i1: usize, i2: usize) -> AABoundingBox {
        let verts = self.vertices.as_ref();
        let p1 = BVHTree::build_vertex(verts, i0);
        let p2 = BVHTree::build_vertex(verts, i1);
        let p3 = BVHTree::build_vertex(verts, i2);
        let bmin = Vector3f::new(p1.x.min(p2.x).min(p3.x), p1.y.min(p2.y).min(p3.y), p1.z.min(p2.z).min(p3.z));
        let bmax = Vector3f::new(p1.x.max(p2.x).max(p3.x), p1.y.max(p2.y).max(p3.y), p1.z.max(p2.z).max(p3.z));
        AABoundingBox::new(&bmin, &bmax)
    }

    fn build_vertex(vertices: &[f64], i: usize) -> Vector3f {
        let base = i * 3;
        Vector3f::new(vertices[base], vertices[base + 1], vertices[base + 2])
    }

    fn tri_centroid(vertex0: &Vector3f, vertex1: &Vector3f, vertex2: &Vector3f) -> Vector3f {
        let mut centroid: Vector3f = *vertex0;
        centroid += vertex1;
        centroid += vertex2;
        centroid *= 1.0 / 3.0;
        centroid
    }

    /// Best split plane for `node` under the surface-area heuristic, or `None` when no plane
    /// separates its triangles.
    ///
    /// Full derivation in `docs/heuristique_aire_surface.md`; in case of doubt about which
    /// area enters the cost, §3 is the section to open. Condensed:
    ///
    /// For two convex volumes B ⊆ A, a uniformly distributed ray that meets A also meets B
    /// with probability SA(B)/SA(A) `[1]`. The expected cost of splitting a node of area A
    /// into children of areas A_L, A_R holding N_L, N_R triangles is therefore
    ///
    /// ```text
    /// C(split) ≈ t_trav + t_isect·(A_L·N_L + A_R·N_R) / A        [3]
    /// ```
    ///
    /// where the approximation is to assume both children will be leaves. A and `t_trav` are
    /// common to every candidate of a node, so minimising `A_L·N_L + A_R·N_R` alone gives the
    /// same winner — which is what this returns, un-normalised, ready to be compared against
    /// `calculate_node_cost` per `[5]`.
    ///
    /// **A_L is the area of the future child's box**, hence of the union of every bin on the
    /// left of the plane. The area of a single bin is the probability of nothing: no step of
    /// the traversal ever asks whether a ray meets bin 3.
    fn find_best_split_plane(&self, node: &BVHNode) -> Option<SplitCandidate> {
        let mut best: Option<SplitCandidate> = None;

        for axis in 0..3 {
            // Bins span the extent of the *centroids*, not the node's box: a plane outside
            // that extent leaves one side empty, so it would waste candidates. `[6]`
            let (centroid_min, centroid_max) = self.centroid_extent(node, axis);
            if centroid_min == centroid_max {
                continue; // every centroid coincides on this axis, no plane separates them
            }

            let bin_width = (centroid_max - centroid_min) / BIN_COUNT as f64;
            let bins = self.fill_bins(node, axis, centroid_min, bin_width);

            // Prefix and suffix scan: at boundary i the left child holds bins 0..=i and the
            // right child bins i+1.., so their areas are those of the two accumulated unions.
            let mut left_areas = [0.0; SPLIT_COUNT];
            let mut right_areas = [0.0; SPLIT_COUNT];
            let mut left_counts = [0; SPLIT_COUNT];
            let mut right_counts = [0; SPLIT_COUNT];

            let mut left_box = AABoundingBox::empty();
            let mut right_box = AABoundingBox::empty();
            let mut left_sum = 0;
            let mut right_sum = 0;

            for i in 0..SPLIT_COUNT {
                left_box.combine_with(&bins[i].bounds);
                left_sum += bins[i].tri_count;
                left_areas[i] = left_box.half_area();
                left_counts[i] = left_sum;

                // The suffix is filled from the far end: boundary `mirrored` is the one whose
                // right side starts at bin `mirrored + 1`.
                let mirrored = SPLIT_COUNT - 1 - i;
                right_box.combine_with(&bins[mirrored + 1].bounds);
                right_sum += bins[mirrored + 1].tri_count;
                right_areas[mirrored] = right_box.half_area();
                right_counts[mirrored] = right_sum;
            }

            // A union can only grow, so the prefix areas cannot decrease and the suffix areas
            // cannot increase. Reading a single bin's area instead of the union breaks this —
            // the invariant is the cheap guard against reintroducing that defect.
            for i in 1..SPLIT_COUNT {
                debug_assert!(left_areas[i] >= left_areas[i - 1], "prefix areas must not decrease");
                debug_assert!(right_areas[i] <= right_areas[i - 1], "suffix areas must not increase");
            }

            for boundary in 0..SPLIT_COUNT {
                // An empty side is not a split. Explicit, rather than left to arithmetic: an
                // empty box reports an area of 0, which would make such a plane look *free*.
                if left_counts[boundary] == 0 || right_counts[boundary] == 0 {
                    continue;
                }

                let cost = left_areas[boundary] * left_counts[boundary] as f64 + right_areas[boundary] * right_counts[boundary] as f64;

                let is_better = match &best {
                    None => true,
                    Some(current) => cost < current.cost,
                };

                if is_better {
                    best = Some(SplitCandidate {
                        axis,
                        boundary,
                        centroid_min,
                        bin_width,
                        cost,
                    });
                }
            }
        }

        best
    }

    /// Range covered by the triangle centroids of `node` along `axis`.
    fn centroid_extent(&self, node: &BVHNode, axis: usize) -> (f64, f64) {
        let mut min = f64::MAX;
        let mut max = f64::MIN;

        for i in node.left_first..(node.left_first + node.tri_count) {
            let centroid_pos = self.tris[self.tri_idx[i]].centroid[axis];
            min = min.min(centroid_pos);
            max = max.max(centroid_pos);
        }

        (min, max)
    }

    /// Accumulates a bounding box and a count into the bin each triangle's centroid falls in.
    ///
    /// Note the asymmetry, which is deliberate: the **centroid** decides the bin, because a
    /// triangle goes entirely to one side of the plane, but the box accumulated is the
    /// triangle's **full bounding box**, which may straddle that plane. The two children
    /// therefore overlap, and must — a child's box has to contain its triangles whole.
    /// See `docs/heuristique_aire_surface.md` §4.
    fn fill_bins(&self, node: &BVHNode, axis: usize, centroid_min: f64, bin_width: f64) -> [Bin; BIN_COUNT] {
        let mut bins = [Bin {
            bounds: AABoundingBox::empty(),
            tri_count: 0,
        }; BIN_COUNT];

        for i in node.left_first..(node.left_first + node.tri_count) {
            let tri = &self.tris[self.tri_idx[i]];
            let bin = &mut bins[bin_index(tri.centroid[axis], centroid_min, bin_width)];
            bin.bounds
                .combine_with(&self.aabound_from_triangle(tri.vertex0_idx, tri.vertex1_idx, tri.vertex2_idx));
            bin.tri_count += 1;
        }

        bins
    }

    /// Cost of keeping `node` as a leaf: `A_node · N`. `[5]`
    ///
    /// The right-hand side of the split test, whose left-hand side is what
    /// `find_best_split_plane` returns. **Neither is divided by `A_node`**, and that shared
    /// convention is what makes the comparison meaningful: `[3] < [4]` multiplied through by
    /// `A_node`, with `t_trav = 0` and `t_isect = 1`. Changing the convention on one side
    /// alone silently redefines "is splitting worth it". Derivation in
    /// `docs/heuristique_aire_surface.md` §2, and the `t_trav = 0` departure in §6.
    fn calculate_node_cost(&self, node: &BVHNode) -> f64 {
        node.bbox.half_area() * node.tri_count as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `side³` small triangles, one per cell of a regular grid, well separated from one
    /// another. Deliberately not axis-aligned: the normal is (0.03, −0.12, 0.15), all three
    /// components non-zero, so no axis-aligned ray lies in a triangle's plane — which
    /// Möller–Trumbore would reject as degenerate, quietly emptying the ray sets below.
    fn unbuilt_grid_mesh(side: usize) -> BVHTree {
        let mut vertices: Vec<f64> = Vec::new();
        let mut indices: Vec<usize> = Vec::new();

        for ix in 0..side {
            for iy in 0..side {
                for iz in 0..side {
                    let (x, y, z) = (ix as f64, iy as f64, iz as f64);
                    let first_vertex = vertices.len() / 3;

                    vertices.extend_from_slice(&[x, y, z]);
                    vertices.extend_from_slice(&[x + 0.4, y + 0.1, z]);
                    vertices.extend_from_slice(&[x + 0.1, y + 0.4, z + 0.3]);

                    indices.extend_from_slice(&[first_vertex, first_vertex + 1, first_vertex + 2]);
                }
            }
        }

        BVHTree::new(Arc::new(vertices), &indices)
    }

    fn grid_mesh(side: usize) -> BVHTree {
        let mut tree = unbuilt_grid_mesh(side);
        tree.build();
        tree
    }

    /// A node spanning `tri_count` triangles from position `left_first` of `tri_idx`.
    ///
    /// Used to hand candidates to `find_best_split_plane` on an unbuilt tree. A built tree
    /// cannot supply them: with `t_trav = 0` splitting always pays, so every leaf ends up
    /// holding a single triangle whose centroid extent is empty on all three axes — no
    /// candidate left anywhere. Any contiguous range of an unbuilt `tri_idx` is a legitimate
    /// node, which is what makes this legitimate rather than a fixture.
    fn node_over(tree: &BVHTree, left_first: usize, tri_count: usize) -> BVHNode {
        let mut node = BVHNode {
            bbox: AABoundingBox::empty(),
            left_first,
            tri_count,
        };
        tree.update_bounds(&mut node);
        node
    }

    /// Nearest intersection found by testing every triangle, ignoring the tree entirely.
    fn brute_force(tree: &BVHTree, ray: &Ray, near: f64, far: f64) -> Option<(f64, usize)> {
        let verts = tree.vertices.as_ref();
        let mut nearest: Option<(f64, usize)> = None;

        for (idx, tri) in tree.tris.iter().enumerate() {
            let p0 = BVHTree::build_vertex(verts, tri.vertex0_idx);
            let p1 = BVHTree::build_vertex(verts, tri.vertex1_idx);
            let p2 = BVHTree::build_vertex(verts, tri.vertex2_idx);

            if let Some(hit) = intersect_ray(ray, &p0, &p1, &p2) {
                if hit.t < near || hit.t > far {
                    continue;
                }
                let closer = match nearest {
                    None => true,
                    Some((t, _)) => hit.t < t,
                };
                if closer {
                    nearest = Some((hit.t, idx));
                }
            }
        }

        nearest
    }

    /// A deterministic ray set that actually reaches the geometry.
    ///
    /// Rays aimed at triangle centroids from three distant origins, so the comparison below is
    /// not vacuous — a lattice of parallel rays mostly misses a mesh of small, well separated
    /// slivers, which is how the first version of this test managed to hit six times out of
    /// two hundred. Aiming at a centroid also lands rays exactly on bin boundaries, the case
    /// that matters most here.
    ///
    /// Axis-aligned pencils are added on purpose rather than out of laziness: they graze the
    /// flat faces of the bins' boxes, which is where a conservative box test and a partition
    /// have to agree. Most of them miss, which is equally worth checking.
    fn ray_set(tree: &BVHTree) -> Vec<Ray> {
        let mut rays = Vec::new();

        let origins = [
            Vector3f::new(-6.0, -7.0, -8.0),
            Vector3f::new(9.0, -6.0, 7.0),
            Vector3f::new(-5.0, 8.0, -9.0),
        ];
        for tri in tree.tris.iter() {
            for origin in origins.iter() {
                rays.push(Ray::new(origin, &(&tri.centroid - origin)));
            }
        }

        for a in 0..7 {
            for b in 0..7 {
                let u = -1.0 + a as f64 * 0.7;
                let v = -1.0 + b as f64 * 0.7;

                rays.push(Ray::new(&Vector3f::new(-5.0, u, v), &Vector3f::new(1.0, 0.0, 0.0)));
                rays.push(Ray::new(&Vector3f::new(u, -5.0, v), &Vector3f::new(0.0, 1.0, 0.0)));
                rays.push(Ray::new(&Vector3f::new(u, v, -5.0), &Vector3f::new(0.0, 0.0, 1.0)));
            }
        }

        rays
    }

    /// The binned prefix/suffix scan must give exactly the cost a full scan of the triangles
    /// gives for the same plane. This is the test that would have caught reading a single
    /// bin's area instead of the union of every bin on that side — see
    /// `docs/heuristique_aire_surface.md` §3.
    ///
    /// Equality is exact, not approximate, and that is a claim about the arithmetic: both
    /// paths union the same boxes with `min`/`max`, which are exact and order-independent, then
    /// evaluate the same expression on the same operands. Any difference is a difference of
    /// substance, never of rounding.
    ///
    /// Scope: it validates the **accumulation**, not the classification — both sides bin with
    /// `bin_index` on purpose, so that the test bears on one thing at a time.
    #[test]
    fn test_binned_cost_matches_exhaustive_scan() {
        let tree = unbuilt_grid_mesh(4);

        // Ranges of varied length and offset, so the check spans candidates with many bins on
        // both sides as well as ones with very few, over all three axes.
        let ranges = [(0, 64), (0, 33), (7, 40), (10, 20), (0, 9), (56, 8), (30, 2)];

        let mut checked = 0;
        for (left_first, tri_count) in ranges.iter() {
            let node = node_over(&tree, *left_first, *tri_count);

            if let Some(split) = tree.find_best_split_plane(&node) {
                assert_eq!(
                    split.cost,
                    tree.exhaustive_split_cost(&node, &split),
                    "binned cost disagrees with the exhaustive scan over {} triangles from {}, axis {} boundary {}",
                    tri_count,
                    left_first,
                    split.axis,
                    split.boundary
                );
                checked += 1;
            }
        }

        assert!(checked >= ranges.len(), "only {} of {} ranges yielded a candidate", checked, ranges.len());
    }

    /// The tree must return exactly what testing every triangle returns.
    ///
    /// This is the real guard on the partition: `bvh_stats` counts work, and a partition that
    /// loses a triangle would simply report *less* work while quietly dropping geometry. Only
    /// a comparison against brute force sees it.
    ///
    /// Distances are compared exactly, because both paths call the same `intersect_ray` on the
    /// same vertices — the tree changes which triangles are tested, never how.
    #[test]
    fn test_query_matches_brute_force() {
        let tree = grid_mesh(3);
        let (near, far) = (0.0001, 1000.0);

        let mut hits = 0;
        for ray in ray_set(&tree).iter() {
            let expected = brute_force(&tree, ray, near, far);
            let actual = tree.query(ray, near, far).map(|(hit, idx)| (hit.t, idx));

            match (expected, actual) {
                (None, None) => {}
                (Some((expected_t, expected_idx)), Some((actual_t, actual_idx))) => {
                    assert_eq!(expected_t, actual_t, "wrong distance for ray {:?}", ray);
                    assert_eq!(expected_idx, actual_idx, "wrong triangle for ray {:?}", ray);
                    hits += 1;
                }
                (expected, actual) => panic!("brute force gave {:?}, the tree gave {:?}, for ray {:?}", expected, actual, ray),
            }
        }

        assert!(hits > 20, "only {} rays hit the mesh; the ray set proves little", hits);
    }

    /// `intersect_p` must say `true` exactly when `query` finds something.
    ///
    /// The two are separate traversals — different stack, no ordering, no `min_t`, an early return
    /// — so nothing but a test keeps them answering the same question. And the failure they would
    /// hide is quiet: an `intersect_p` that missed occluders would not crash or slow anything
    /// down, it would simply let light through walls.
    ///
    /// Bounded intervals are checked as well as open ones, because that is the shadow-ray case:
    /// `far` carries the distance to the light, and a hit beyond it must not count.
    #[test]
    fn test_intersect_p_agrees_with_query() {
        let tree = grid_mesh(3);
        let (near, far) = (0.0001, 1000.0);

        let mut occluded = 0;
        for ray in ray_set(&tree).iter() {
            let nearest = tree.query(ray, near, far);
            assert_eq!(
                tree.intersect_p(ray, near, far),
                nearest.is_some(),
                "disagreement over the full interval for ray {:?}",
                ray
            );

            // Then sweep `far` across the nearest hit. The factors either side of 1 matter most:
            // a leaf's box is entered slightly before its triangle, so a bound falling between the
            // two is the only case that exercises the interval check *inside* the leaf rather than
            // the box test that usually shields it. Cutting at half the distance, which is the
            // obvious thing to write, is rejected by the box test and proves nothing about it.
            if let Some((hit, _)) = nearest {
                occluded += 1;

                for factor in [0.5, 0.99, 0.999999, 1.0, 1.000001] {
                    let bound = hit.t * factor;
                    assert_eq!(
                        tree.intersect_p(ray, near, bound),
                        tree.query(ray, near, bound).is_some(),
                        "disagreement at {} of the nearest hit ({}) for ray {:?}",
                        factor,
                        hit.t,
                        ray
                    );
                }
            }
        }

        assert!(occluded > 20, "only {} rays hit the mesh; the agreement proves little", occluded);
    }

    /// Well separated triangles must end up in distinct leaves. This is the assertion that
    /// fails on the defect this replaces: with the cost read per bin, one leaf kept a third of
    /// the mesh.
    #[test]
    fn test_separated_triangles_land_in_distinct_leaves() {
        let tree = grid_mesh(4);
        let stats = tree.build_stats();

        assert_eq!(
            stats.total_leaf_tri_count,
            tree.triangle_count(),
            "every triangle belongs to exactly one leaf"
        );
        assert!(
            stats.max_leaf_tri_count <= 4,
            "64 well separated triangles should not share a leaf; largest holds {}",
            stats.max_leaf_tri_count
        );
    }
}
