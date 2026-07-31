use super::triangle_mesh::{intersect_ray, TriangleIntersection};
use crate::geom::aabound::AABoundingBox;
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use std::sync::Arc;
use std::vec;

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
    /// Nodes taken off the traversal stack.
    pub nodes_visited: usize,

    /// Ray/box tests. Deliberately counted apart from `nodes_visited`, because the
    /// traversal does not perform one test per visited node: it tests a child's box
    /// before pushing it and tests the same box again after popping it. The two figures
    /// are not interchangeable, and only this one reflects the cost actually paid.
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

struct BVHNode {
    bbox: AABoundingBox,
    left_first: usize,
    tri_count: usize,
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
    next_node_idx: usize,
}

#[derive(Copy, Clone)]
struct Bin {
    bounds: AABoundingBox,
    tri_count: usize,
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
            next_node_idx: 0,
        }
    }

    pub fn build(&mut self) {
        let mut root = BVHNode {
            bbox: AABoundingBox::new_invalid(),
            left_first: 0,
            tri_count: self.tris.len(),
        };
        self.update_bounds(&mut root);
        self.nodes.push(root);
        self.next_node_idx = 1;
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
        let node = &self.nodes[node_idx];

        // Find the best split plane for the current node
        let (best_axis, best_pos, best_cost) = self.find_best_split_plane(node);

        // Check that splitting is worth it
        let current_cost = self.calculate_node_cost(node);
        if best_cost >= current_cost {
            return; // splitting is not worth it
        }

        // Clone node data needed for split search
        // Do not keep a reference to the node, as it may be moved in memory
        // (and thus compiler does not allow mutable borrow)
        let (left_first, tri_count, _bbox) = {
            let node = &self.nodes[node_idx];
            (node.left_first, node.tri_count, node.bbox)
        };

        // In place triangle set partitioning with respect to the best axis and position
        let mut i = left_first;
        let mut j = i + tri_count - 1;
        while i <= j && j != usize::MAX {
            if self.tris[self.tri_idx[i]].centroid[best_axis] < best_pos {
                i += 1;
            }
            else {
                self.tri_idx.swap(i, j);
                j -= 1;
            }
        }

        // Do not split if one side is empty
        let left_count = i - left_first;
        if left_count == 0 || left_count == tri_count {
            return;
        }

        // Create left and right nodes
        let mut left_node = BVHNode {
            bbox: AABoundingBox::new_invalid(),
            left_first: left_first,
            tri_count: left_count,
        };
        self.update_bounds(&mut left_node);
        let left_node_idx = self.next_node_idx;
        self.nodes.push(left_node);
        self.next_node_idx += 1;

        let mut right_node = BVHNode {
            bbox: AABoundingBox::new_invalid(),
            left_first: i,
            tri_count: tri_count - left_count,
        };
        self.update_bounds(&mut right_node);
        self.nodes.push(right_node);
        self.next_node_idx += 1;

        // Now, get a fresh mutable reference to the parent node and update it
        let node = &mut self.nodes[node_idx];
        node.left_first = left_node_idx;
        node.tri_count = 0;

        self.subdivide(left_node_idx);
        self.subdivide(left_node_idx + 1);
    }

    fn evaluate_sah(&self, node: &BVHNode, axis: usize, pos: f64) -> f64 {
        let mut left_box: AABoundingBox = AABoundingBox::new_invalid();
        let mut right_box: AABoundingBox = AABoundingBox::new_invalid();
        let mut left_count: usize = 0;
        let mut right_count: usize = 0;
        for i in node.left_first..(node.left_first + node.tri_count) {
            let tri = &self.tris[self.tri_idx[i]];
            if tri.centroid[axis] < pos {
                left_count += 1;
                left_box.combine_with(&self.aabound_from_triangle(tri.vertex0_idx, tri.vertex1_idx, tri.vertex2_idx));
            }
            else {
                right_count += 1;
                right_box.combine_with(&self.aabound_from_triangle(tri.vertex0_idx, tri.vertex1_idx, tri.vertex2_idx));
            }
        }

        let cost = left_box.half_area() * left_count as f64 + right_box.half_area() * right_count as f64;
        if cost > 0.0 {
            cost
        }
        else {
            f64::MAX // Avoid division by zero
        }
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

    /// Tests `node`'s bounding box against `ray` and records the test.
    ///
    /// Every ray/box test in the traversal goes through here, so `box_tests` cannot drift
    /// out of step with the tests actually performed.
    fn hit_box(node: &BVHNode, ray: &Ray, near: f64, far: f64, stats: &mut TraversalStats) -> Option<f64> {
        stats.box_tests += 1;
        node.bbox.hit(ray, near, far)
    }

    fn traverse(&self, ray: &Ray, near: f64, far: f64, stats: &mut TraversalStats) -> Option<(TriangleIntersection, usize)> {
        let verts = self.vertices.as_ref();
        let mut min_t: f64 = f64::MAX;
        let mut current_intersection: Option<TriangleIntersection> = None;
        let mut current_tri_idx: usize = 0;
        let mut stack: Vec<usize> = Vec::with_capacity(64);
        stack.push(0);

        while let Some(node_idx) = stack.pop() {
            stats.nodes_visited += 1;
            let node = &self.nodes[node_idx];
            if let Some(t) = BVHTree::hit_box(node, ray, near, far, stats) {
                if t >= min_t {
                    continue;
                }

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
                    let left_node = &self.nodes[node.left_first];
                    let right_node = &self.nodes[node.left_first + 1];
                    let left_hit = BVHTree::hit_box(left_node, ray, near, far, stats);
                    let right_hit = BVHTree::hit_box(right_node, ray, near, far, stats);
                    match (&left_hit, &right_hit) {
                        (Some(ref d1), Some(ref d2)) => {
                            if *d1 < *d2 {
                                if *d2 < min_t {
                                    stack.push(node.left_first + 1);
                                }
                                if *d1 < min_t {
                                    stack.push(node.left_first);
                                }
                            }
                            else {
                                if *d1 < min_t {
                                    stack.push(node.left_first);
                                }
                                if *d2 < min_t {
                                    stack.push(node.left_first + 1);
                                }
                            }
                        }
                        (Some(ref d), None) if *d < min_t => stack.push(node.left_first),
                        (Some(_), None) => continue,
                        (None, Some(ref d)) if *d < min_t => stack.push(node.left_first + 1),
                        (None, Some(_)) => continue,
                        (None, None) => continue,
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

    /// Finds the best split plane for the given node.
    /// Returns (best_axis, best_pos, best_cost).
    fn find_best_split_plane(&self, node: &BVHNode) -> (usize, f64, f64) {
        let mut best_cost = f64::MAX;
        let mut best_axis = 0;
        let mut best_pos = 0.0;

        for axis in 0..3 {
            let mut bound_min: f64 = f64::MAX;
            let mut bound_max: f64 = f64::MIN;
            for i in node.left_first..(node.left_first + node.tri_count) {
                let tri = &self.tris[self.tri_idx[i]];
                let candidate_pos = tri.centroid[axis];
                bound_min = bound_min.min(candidate_pos);
                bound_max = bound_max.max(candidate_pos);
            }
            if bound_min == bound_max {
                continue; // No valid split
            }

            let scale = (bound_max - bound_min) / 8.0;
            let mut bins: [Bin; 8] = [Bin {
                bounds: AABoundingBox::new_invalid(),
                tri_count: 0,
            }; 8];

            for i in node.left_first..(node.left_first + node.tri_count) {
                let tri = &self.tris[self.tri_idx[i]];
                let candidate_pos = tri.centroid[axis];
                let bin_idx = (((candidate_pos - bound_min) / scale).floor() as usize).clamp(0, 7);
                let bin = &mut bins[bin_idx];
                bin.bounds
                    .combine_with(&self.aabound_from_triangle(tri.vertex0_idx, tri.vertex1_idx, tri.vertex2_idx));
                bin.tri_count += 1;
            }

            let mut left_areas = vec![0.0; 7];
            let mut right_areas = vec![0.0; 7];
            let mut left_count = vec![0; 7];
            let mut right_count = vec![0; 7];
            let mut left_box: AABoundingBox = AABoundingBox::new_invalid();
            let mut right_box: AABoundingBox = AABoundingBox::new_invalid();
            let mut left_sum = 0;
            let mut right_sum = 0;
            for i in 0..7 {
                let left_bin = &bins[i];
                left_sum += left_bin.tri_count;
                left_count[i] = left_sum;
                left_box.combine_with(&left_bin.bounds);
                left_areas[i] = left_bin.bounds.half_area();

                let right_bin = &bins[7 - i];
                right_sum += right_bin.tri_count;
                right_count[6 - i] = right_sum;
                right_box.combine_with(&right_bin.bounds);
                right_areas[6 - i] = right_bin.bounds.half_area();
            }

            let inv_scale = (bound_max - bound_min) / 8.0;
            for i in 0..7 {
                let plane_cost = left_areas[i] * left_count[i] as f64 + right_areas[i] * right_count[i] as f64;
                if plane_cost < best_cost {
                    best_cost = plane_cost;
                    best_axis = axis;
                    best_pos = bound_min + (i as f64) * inv_scale;
                }
            }
        }
        (best_axis, best_pos, best_cost)
    }

    /// Calculates the cost of the current node by node_idx.
    fn calculate_node_cost(&self, node: &BVHNode) -> f64 {
        let current_area = node.bbox.half_area();
        current_area * node.tri_count as f64
    }
}
