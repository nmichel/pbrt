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

pub struct ThinLensCameraNode {
    pub pos: Vector3f,
    pub look: Vector3f,
    pub up: Vector3f,
    pub radius: f64,
    pub focal_length: f64,
}

impl CameraNode for ThinLensCameraNode {}

impl ThinLensCameraNode {
    pub fn new(pos: Vector3f, look: Vector3f, up: Vector3f, radius: f64, focal_length: f64) -> Self {
        Self {
            pos,
            look,
            up,
            radius,
            focal_length,
        }
    }
}

impl Node for ThinLensCameraNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        visitor.visit_camera_thin_lens(self);
    }
}
