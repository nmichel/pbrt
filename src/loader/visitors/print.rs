use super::super::ast::*;
use super::Visitor;

pub struct PrintVisitor {
    stack: Vec<String>,
}

impl PrintVisitor {
    pub fn new() -> Self {
        PrintVisitor { stack: Vec::new() }
    }

    pub fn visit(self: &mut Self, node: &SceneNode) {
        node.visit(self);
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
