use std::sync::Arc;

use crate::materials::{Lambertian, Material};
use crate::objects::{Object, Simple};
use crate::scene::Scene;
use crate::shapes::{Shape, Sphere};
use crate::spectrum::Spectrum;
use crate::textures::{PlainColor, Texture};

pub trait Node {
    fn visit(self: &Self, visitor: &mut dyn Visitor);
}

pub trait MaterialNode: Node {}
pub trait ObjectNode: Node {}
pub trait ShapeNode: Node {}
pub trait TextureNode: Node {}

pub trait Visitor {
    fn visit_scene(self: &mut Self, node: &SceneNode);
    fn visit_object_simple(self: &mut Self, node: &ObjectSimpleNode);
    fn visit_shape_sphere(self: &mut Self, node: &SphereShapeNode);
    fn visit_material_lambertian(self: &mut Self, node: &LambertianMaterialNode);
    fn visit_texture_color(self: &mut Self, node: &ColorTextureNode);
}

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

// Visitors

pub struct PrintVisitor {
    stack: Vec<String>,
}

impl PrintVisitor {
    pub fn new() -> Self {
        PrintVisitor { stack: Vec::new() }
    }
}

impl Visitor for PrintVisitor {
    fn visit_scene(self: &mut Self, node: &SceneNode) {
        let r = self.stack.iter().fold(String::new(), |a, s| a + s);

        println!("scene {}", r);
    }

    fn visit_object_simple(self: &mut Self, node: &ObjectSimpleNode) {
        let material = self.stack.pop().unwrap();
        let shape = self.stack.pop().unwrap();
        self.stack.push(format!("object simple {} {}", shape, material));
    }

    fn visit_shape_sphere(self: &mut Self, node: &SphereShapeNode) {
        self.stack.push(format!("sphere {}", node.radius));
    }

    fn visit_material_lambertian(self: &mut Self, node: &LambertianMaterialNode) {
        let texture = self.stack.pop().unwrap();
        self.stack.push(format!("lambertian {}", texture));
    }

    fn visit_texture_color(self: &mut Self, node: &ColorTextureNode) {
        self.stack.push(format!("color {:?}", node.color));
    }
}

pub struct SceneBuilderVisitor {
    scene: Scene,
    objects: Vec<Arc<dyn Object>>,
    shapes: Vec<Arc<dyn Shape>>,
    materials: Vec<Arc<dyn Material>>,
    textures: Vec<Arc<dyn Texture>>,
}

impl SceneBuilderVisitor {
    pub fn new() -> Self {
        SceneBuilderVisitor {
            scene: Scene::new(),
            objects: Vec::new(),
            shapes: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
        }
    }
}

impl Visitor for SceneBuilderVisitor {
    fn visit_scene(self: &mut Self, node: &SceneNode) {
        while let Some(object) = self.objects.pop() {
            self.scene.add_object(object);
        }
    }

    fn visit_object_simple(self: &mut Self, node: &ObjectSimpleNode) {
        let material = self.materials.pop().unwrap();
        let shape = self.shapes.pop().unwrap();
        self.objects.push(Arc::new(Simple::new(shape, material)));
    }

    fn visit_shape_sphere(self: &mut Self, node: &SphereShapeNode) {
        self.shapes.push(Arc::new(Sphere::new(node.radius)));
    }

    fn visit_material_lambertian(self: &mut Self, node: &LambertianMaterialNode) {
        let texture = self.textures.pop().unwrap();
        self.materials.push(Arc::new(Lambertian::new(texture)));
    }

    fn visit_texture_color(self: &mut Self, node: &ColorTextureNode) {
        self.textures.push(Arc::new(PlainColor::new(node.color)));
    }
}
