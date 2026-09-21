use crate::loader::visitors::Visitor;

use super::{MaterialNode, Node, ShapeNode, TransformNode};

pub trait ObjectNode: Node {}

pub struct ObjectSimpleNode {
    pub shape: Box<dyn ShapeNode>,
    pub material: Box<dyn MaterialNode>,
}

impl ObjectNode for ObjectSimpleNode {}

impl Node for ObjectSimpleNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        self.shape.visit(visitor);
        self.material.visit(visitor);
        visitor.visit_object_simple(self);
    }
}

impl ObjectSimpleNode {
    pub fn new(shape: Box<dyn ShapeNode>, material: Box<dyn MaterialNode>) -> Self {
        ObjectSimpleNode { shape, material }
    }
}

pub struct ObjectTransformedNode {
    pub object: Box<dyn ObjectNode>,
    pub transform: Box<TransformNode>,
}

impl ObjectNode for ObjectTransformedNode {}

impl Node for ObjectTransformedNode {
    /// The transformation is visited **before** the object it places.
    ///
    /// This order is what makes a current transformation matrix possible: a visitor building a
    /// scene learns the placement first, so the geometry underneath can be constructed already
    /// placed. Visited the other way round, the placement would only be known once the thing to
    /// place was finished and closed, which is the shape of the problem rather than its solution.
    ///
    /// The pair of calls around the child is the block's scope: `enter_object_transformed` opens
    /// it, `leave_object_transformed` closes it.
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        self.transform.visit(visitor);
        visitor.enter_object_transformed(self);
        self.object.visit(visitor);
        visitor.leave_object_transformed(self);
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
