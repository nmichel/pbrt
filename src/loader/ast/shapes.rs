use crate::loader::visitors::Visitor;

use super::Node;

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
