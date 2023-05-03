mod objects;
mod transform;

pub use self::objects::*;
pub use self::transform::*;

use crate::geom::vector3::Vector3f;
use crate::spectrum::Spectrum;

use super::visitors::Visitor;

pub trait Node {
    fn visit(self: &Self, visitor: &mut dyn Visitor);
}

pub trait MaterialNode: Node {}
pub trait ObjectNode: Node {}
pub trait ShapeNode: Node {}
pub trait TextureNode: Node {}

// Scene

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

// Objects

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

// Shapes

pub struct SphereShapeNode {
    pub radius: f64,
}

impl ShapeNode for SphereShapeNode {}

impl Node for SphereShapeNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        visitor.visit_shape_sphere(self);
    }
}

impl SphereShapeNode {
    pub fn new(radius: f64) -> Self {
        SphereShapeNode { radius }
    }
}

// Material

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

// Texture

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
