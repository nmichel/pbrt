use crate::loader::visitors::Visitor;

use super::{Node, ObjectNode};

pub struct SceneNode {
    pub objects: Vec<Box<dyn ObjectNode>>,
}

impl SceneNode {
    pub fn new() -> Self {
        SceneNode { objects: Vec::new() }
    }

    pub fn add_object(self: &mut Self, object: Box<dyn ObjectNode>) {
        self.objects.push(object);
    }
}

impl Node for SceneNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        for object in &self.objects {
            object.visit(visitor);
        }
        visitor.visit_scene(self);
    }
}
