use std::sync::Arc;

use super::super::ast::*;
use super::Visitor;
use crate::materials::{Lambertian, Material};
use crate::objects::{Object, Simple};
use crate::scene::Scene;
use crate::shapes::{Shape, Sphere};
use crate::textures::{PlainColor, Texture};

pub struct SceneBuilderVisitor {
    pub scene: Scene,
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

    pub fn visit(self: &mut Self, node: &SceneNode) {
        node.visit(self);
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
