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
