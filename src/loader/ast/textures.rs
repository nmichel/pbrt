use crate::loader::visitors::Visitor;
use crate::spectrum::Spectrum;

use super::Node;

pub trait TextureNode: Node {}

pub struct ColorTextureNode {
    pub color: Spectrum,
}

impl TextureNode for ColorTextureNode {}

impl Node for ColorTextureNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        visitor.visit_texture_color(self);
    }
}

impl ColorTextureNode {
    pub fn new(color: Spectrum) -> Self {
        ColorTextureNode { color }
    }
}
