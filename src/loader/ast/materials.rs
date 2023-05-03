use crate::loader::visitors::Visitor;

use super::{Node, TextureNode};

pub trait MaterialNode: Node {}

pub struct LambertianMaterialNode {
    pub texture: Box<dyn TextureNode>,
}

impl MaterialNode for LambertianMaterialNode {}

impl Node for LambertianMaterialNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        self.texture.visit(visitor);
        visitor.visit_material_lambertian(self);
    }
}

impl LambertianMaterialNode {
    pub fn new(texture: Box<dyn TextureNode>) -> Self {
        LambertianMaterialNode { texture }
    }
}
