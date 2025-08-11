use super::triangle_mesh::{intersect_ray, TriangleIntersection};
use crate::geom::aabound::AABoundingBox;
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use std::sync::Arc;

pub trait Accumulator {
    fn accumulate(&mut self, items: &Vec<usize>) -> ();
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
        // Clone node data needed for split search
        // Do not keep a reference to the node, as it may be moved in memory
        // (and thus compiler does not allow mutable borrow)
        let (left_first, tri_count, bbox) = {
            let node = &self.nodes[node_idx];
            (node.left_first, node.tri_count, node.bbox)
        };

        let mut best_axis = 0;
        let mut best_pos = 0.0;
        let mut best_cost = f64::MAX;

        // Extensively search for the best split
        for axis in 0..3 {
            for i in left_first..(left_first + tri_count) {
                let tri = &self.tris[self.tri_idx[i]];
                let candidate_pos = tri.centroid[axis];
                let cost = self.evaluate_sah(node_idx, axis, candidate_pos);
                if cost < best_cost {
                    best_cost = cost;
                    best_axis = axis;
                    best_pos = candidate_pos;
                }
            }
        }

        // Check that splitting is worth it
        let current_area = bbox.half_area();
        let current_cost = current_area * tri_count as f64;
        if best_cost >= current_cost {
            return; // splitting is not worth it
        }

        // In place triangle set partitioning with respect to the best axis and position
        let mut i = left_first;
        let mut j = i + tri_count - 1;
        while i <= j {
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

    fn evaluate_sah(&self, node_idx: usize, axis: usize, pos: f64) -> f64 {
        let node = &self.nodes[node_idx];

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

    pub fn query(&self, ray: &Ray, near: f64, far: f64) -> Option<(TriangleIntersection, usize)> {
        let verts = self.vertices.as_ref();
        let mut min_t: f64 = f64::MAX;
        let mut current_intersection: Option<TriangleIntersection> = None;
        let mut current_tri_idx: usize = 0;
        let mut stack: Vec<usize> = Vec::with_capacity(64);
        stack.push(0);

        while let Some(node_idx) = stack.pop() {
            let node = &self.nodes[node_idx];
            if let Some(t) = node.bbox.hit(ray, near, far) {
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
                    let left_hit = left_node.bbox.hit(ray, near, far);
                    let right_hit = right_node.bbox.hit(ray, near, far);
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
}
