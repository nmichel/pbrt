use crate::geom::vector3::Vector3f;
use crate::loader::visitors::Visitor;

use super::{Node, TransformNode};

pub trait ShapeNode: Node {}

pub struct SphereShapeNode {
    pub radius: f64,
}

impl ShapeNode for SphereShapeNode {}

impl Node for SphereShapeNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        visitor.visit_shape_sphere(self);
    }
}

impl SphereShapeNode {
    pub fn new(radius: f64) -> Self {
        SphereShapeNode { radius }
    }
}

pub struct RectangleShapeNode {
    pub half_width: f64,
    pub half_height: f64,
}

impl ShapeNode for RectangleShapeNode {}

impl Node for RectangleShapeNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        visitor.visit_shape_rectangle(self);
    }
}

impl RectangleShapeNode {
    pub fn new(half_width: f64, half_height: f64) -> Self {
        RectangleShapeNode { half_width, half_height }
    }
}

pub struct PlaneShapeNode {}

impl ShapeNode for PlaneShapeNode {}

impl Node for PlaneShapeNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        visitor.visit_shape_plane(self);
    }
}

impl PlaneShapeNode {
    pub fn new() -> Self {
        PlaneShapeNode {}
    }
}

pub struct CylinderShapeNode {
    pub radius: f64,
    pub height: f64,
}

impl ShapeNode for CylinderShapeNode {}

impl Node for CylinderShapeNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        visitor.visit_shape_cylinder(self);
    }
}

impl CylinderShapeNode {
    pub fn new(radius: f64, height: f64) -> Self {
        CylinderShapeNode { radius, height }
    }
}

pub struct AABoxShapeNode {
    pub extend: Vector3f,
}

impl ShapeNode for AABoxShapeNode {}

impl Node for AABoxShapeNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        visitor.visit_shape_aabox(self);
    }
}

impl AABoxShapeNode {
    pub fn new(extend: Vector3f) -> Self {
        AABoxShapeNode { extend }
    }
}

pub struct MeshShapeNode {
    pub filename: String,
}

impl ShapeNode for MeshShapeNode {}

impl Node for MeshShapeNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        visitor.visit_shape_mesh(self);
    }
}

impl MeshShapeNode {
    pub fn new(filename: String) -> Self {
        MeshShapeNode { filename: filename }
    }
}

// csg

pub struct CSGShapeElemNode {
    pub shape: Box<dyn ShapeNode>,
    pub transform: Box<TransformNode>,
}

impl ShapeNode for CSGShapeElemNode {}

impl Node for CSGShapeElemNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        self.shape.visit(visitor);
        self.transform.visit(visitor);
        visitor.visit_shape_csg_elem(self);
    }
}

impl CSGShapeElemNode {
    pub fn new(shape: Box<dyn ShapeNode>, transform: Box<TransformNode>) -> Self {
        CSGShapeElemNode { shape, transform }
    }
}

pub struct CSGShapeIntersectionNode {
    pub elems: Vec<Box<CSGShapeElemNode>>,
}

impl ShapeNode for CSGShapeIntersectionNode {}

impl Node for CSGShapeIntersectionNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        for elem in &self.elems {
            elem.visit(visitor);
        }
        visitor.visit_shape_csg_intersection(self);
    }
}

impl CSGShapeIntersectionNode {
    pub fn new(elems: Vec<Box<CSGShapeElemNode>>) -> Self {
        CSGShapeIntersectionNode { elems }
    }
}

pub struct CSGShapeUnionNode {
    pub elems: Vec<Box<CSGShapeElemNode>>,
}

impl ShapeNode for CSGShapeUnionNode {}

impl Node for CSGShapeUnionNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        for elem in &self.elems {
            elem.visit(visitor);
        }
        visitor.visit_shape_csg_union(self);
    }
}

impl CSGShapeUnionNode {
    pub fn new(elems: Vec<Box<CSGShapeElemNode>>) -> Self {
        CSGShapeUnionNode { elems }
    }
}

pub struct CSGShapeSubstractionNode {
    pub elems: Vec<Box<CSGShapeElemNode>>,
}

impl ShapeNode for CSGShapeSubstractionNode {}

impl Node for CSGShapeSubstractionNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        for elem in &self.elems {
            elem.visit(visitor);
        }
        visitor.visit_shape_csg_substraction(self);
    }
}

impl CSGShapeSubstractionNode {
    pub fn new(elems: Vec<Box<CSGShapeElemNode>>) -> Self {
        CSGShapeSubstractionNode { elems }
    }
}
