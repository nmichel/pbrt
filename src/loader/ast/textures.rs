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

pub struct CheckerboardTextureNode {
    pub color1: Spectrum,
    pub color2: Spectrum,
    pub scale: f64,
}

impl TextureNode for CheckerboardTextureNode {}

impl Node for CheckerboardTextureNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        visitor.visit_texture_checkerboard(self);
    }
}

impl CheckerboardTextureNode {
    pub fn new(color1: Spectrum, color2: Spectrum, scale: f64) -> Self {
        CheckerboardTextureNode { color1, color2, scale }
    }
}
