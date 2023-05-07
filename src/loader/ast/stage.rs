use crate::loader::visitors::Visitor;

use super::{CameraNode, Node, SceneNode};

pub struct StageNode {
    pub scene: SceneNode,
    pub camera: Box<dyn CameraNode>,
}

impl StageNode {
    pub fn new(scene: SceneNode, camera: Box<dyn CameraNode>) -> Self {
        StageNode { scene, camera }
    }
}

impl Node for StageNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        self.scene.visit(visitor);
        self.camera.visit(visitor);
        visitor.visit_stage(self);
    }
}
