use crate::loader::visitors::Visitor;

use super::{Node, ObjectNode, TransformNode};

pub struct ObjectTransformedNode {
    pub object: Box<dyn ObjectNode>,
    pub transform: Box<TransformNode>,
}

impl ObjectNode for ObjectTransformedNode {}

impl Node for ObjectTransformedNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        self.object.visit(visitor);
        self.transform.visit(visitor);
        visitor.visit_object_transformed(self);
    }
}

impl ObjectTransformedNode {
    pub fn new(object: Box<dyn ObjectNode>, transform: Box<TransformNode>) -> Self {
        Self { object, transform }
    }
}

pub struct ObjectCompoundNode {
    pub objects: Vec<Box<dyn ObjectNode>>,
}

impl ObjectNode for ObjectCompoundNode {}

impl Node for ObjectCompoundNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        for object in &self.objects {
            object.visit(visitor);
        }
        visitor.visit_object_compound(self);
    }
}

impl ObjectCompoundNode {
    pub fn new() -> Self {
        Self { objects: Vec::new() }
    }

    pub fn add_object(self: &mut Self, object: Box<dyn ObjectNode>) {
        self.objects.push(object);
    }
}
