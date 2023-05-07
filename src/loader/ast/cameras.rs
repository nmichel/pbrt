use crate::geom::vector3::Vector3f;
use crate::loader::visitors::Visitor;

use super::Node;

pub trait CameraNode: Node {}

pub struct PinHoleCameraNode {
    pub pos: Vector3f,
    pub look: Vector3f,
    pub up: Vector3f,
}

impl CameraNode for PinHoleCameraNode {}

impl PinHoleCameraNode {
    pub fn new(pos: Vector3f, look: Vector3f, up: Vector3f) -> Self {
        Self { pos, look, up }
    }
}

impl Node for PinHoleCameraNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        visitor.visit_camera_pin_hole(self);
    }
}
