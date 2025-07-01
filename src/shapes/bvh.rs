use crate::geom::aabound::AABoundingBox;
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use std::cmp::Ordering;

pub trait Accumulator {
    fn accumulate(&mut self, items: &Vec<usize>) -> ();
}

enum BVHNode {
    Internal {
        left: Box<BVHNode>,
        right: Box<BVHNode>,
        bbox: AABoundingBox,
    },
    Leaf {
        indices: Vec<usize>,
        bbox: AABoundingBox,
    },
}

impl BVHNode {
    pub fn get_bbox(&self) -> &AABoundingBox {
        match self {
            BVHNode::Leaf { bbox, .. } => bbox,
            BVHNode::Internal { bbox, .. } => bbox,
        }
    }

    pub fn query(&self, ray: &Ray, near: f64, far: f64, accumulator: &mut dyn Accumulator) -> () {
        if !self.hit_box(ray, near, far) {
            return;
        }

        match self {
            BVHNode::Leaf { indices, .. } => {
                accumulator.accumulate(indices);
            }
            BVHNode::Internal { left, right, .. } => {
                left.query(ray, near, far, accumulator);
                right.query(ray, near, far, accumulator);
            }
        }
    }

    fn hit_box(&self, ray: &Ray, near: f64, far: f64) -> bool {
        self.get_bbox().hit(ray, near, far)
    }
}

pub struct BVHTree {
    pub root: BVHNode,
}

#[derive(Clone, Copy)]
struct Item {
    triangle_index: usize,
    aabound: AABoundingBox,
    centroid: Vector3f,
}

impl BVHTree {
    pub fn new(vertices: &Vec<f64>, indices: &Vec<usize>) -> Self {
        assert!(indices.len() % 3 == 0);

        let triangle_count = indices.len() / 3;
        let mut items = Vec::with_capacity(triangle_count);

        for i in 0..triangle_count {
            let base = i * 3;
            let i0 = indices[base] as usize;
            let i1 = indices[base + 1] as usize;
            let i2 = indices[base + 2] as usize;

            let aabound = BVHTree::aabound_from_triangle(&vertices, i0, i1, i2);
            let centroid = aabound.centroid();
            items.push(Item {
                triangle_index: base,
                aabound,
                centroid,
            });
        }

        let root = BVHTree::build_node(items, 0);

        BVHTree { root }
    }

    pub fn query(&self, ray: &Ray, near: f64, far: f64, accumulator: &mut dyn Accumulator) -> () {
        self.root.query(ray, near, far, accumulator)
    }

    fn aabound_from_triangle(vertices: &[f64], i0: usize, i1: usize, i2: usize) -> AABoundingBox {
        let p1 = BVHTree::get_vertex(vertices, i0);
        let p2 = BVHTree::get_vertex(vertices, i1);
        let p3 = BVHTree::get_vertex(vertices, i2);
        let bmin = Vector3f::new(p1.x.min(p2.x).min(p3.x), p1.y.min(p2.y).min(p3.y), p1.z.min(p2.z).min(p3.z));
        let bmax = Vector3f::new(p1.x.max(p2.x).max(p3.x), p1.y.max(p2.y).max(p3.y), p1.z.max(p2.z).max(p3.z));
        AABoundingBox::new(&bmin, &bmax)
    }

    fn get_vertex(vertices: &[f64], i: usize) -> Vector3f {
        let base = i * 3;
        Vector3f::new(vertices[base], vertices[base + 1], vertices[base + 2])
    }

    fn build_node(mut items: Vec<Item>, depth: usize) -> BVHNode {
        if items.len() <= 2 {
            let indices = items.iter().map(|Item { triangle_index, .. }| *triangle_index).collect::<Vec<_>>();
            let bbox = items
                .iter()
                .map(|Item { aabound, .. }| *aabound)
                .reduce(|a, b| AABoundingBox::combine(&a, &b))
                .unwrap();
            return BVHNode::Leaf { indices, bbox };
        }
        else {
            items.sort_by(Self::choose_comparator(depth));

            let mid = items.len() / 2;
            let left_items = items[..mid].to_vec();
            let right_items = items[mid..].to_vec();

            let left = Box::new(BVHTree::build_node(left_items, depth + 1));
            let right = Box::new(BVHTree::build_node(right_items, depth + 1));

            let bbox = AABoundingBox::combine(left.get_bbox(), right.get_bbox());

            BVHNode::Internal { left, right, bbox }
        }
    }

    fn choose_comparator(depth: usize) -> fn(&Item, &Item) -> Ordering {
        match depth % 3 {
            0 => Self::compare_x,
            1 => Self::compare_y,
            _ => Self::compare_z,
        }
    }

    fn compare_x(a: &Item, b: &Item) -> Ordering {
        a.centroid.x.partial_cmp(&b.centroid.x).unwrap()
    }

    fn compare_y(a: &Item, b: &Item) -> Ordering {
        a.centroid.y.partial_cmp(&b.centroid.y).unwrap()
    }

    fn compare_z(a: &Item, b: &Item) -> Ordering {
        a.centroid.z.partial_cmp(&b.centroid.z).unwrap()
    }
}
