use super::Node;
use crate::geom::vector3::Vector3f;
use crate::loader::visitors::Visitor;

pub trait TransformStepNode: Node {}

pub struct TransformTranslateNode {
    pub offset: Vector3f,
}

impl TransformStepNode for TransformTranslateNode {}

impl Node for TransformTranslateNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        visitor.visit_transform_translate(self);
    }
}

impl TransformTranslateNode {
    pub fn new(offset: Vector3f) -> Self {
        Self { offset }
    }
}

#[derive(Debug)]
pub enum Axis {
    X,
    Y,
    Z,
}

pub struct TransformRotateAxisNode {
    pub angle: f64,
    pub axis: Axis,
}

impl TransformStepNode for TransformRotateAxisNode {}

impl Node for TransformRotateAxisNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        visitor.visit_transform_rotate(self);
    }
}

impl TransformRotateAxisNode {
    pub fn new(angle: f64, axis: Axis) -> Self {
        Self { angle, axis }
    }
}

pub struct TransformNode {
    pub steps: Vec<Box<dyn TransformStepNode>>,
}

impl Node for TransformNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        for object in &self.steps {
            object.visit(visitor);
        }
        visitor.visit_transform(self);
    }
}

impl TransformNode {
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn add_step(self: &mut Self, step: Box<dyn TransformStepNode>) {
        self.steps.push(step);
    }
}
