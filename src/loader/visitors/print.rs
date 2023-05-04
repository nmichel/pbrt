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
        let mut r = String::new();
        for _ in 1..=node.objects.len() {
            let object = self.stack.pop().unwrap();
            r = r + &object;
        }

        println!("scene {{ {} }}", r);
    }

    fn visit_object_compound(self: &mut Self, node: &ObjectCompoundNode) {
        let mut r = String::new();
        for _ in 1..=node.objects.len() {
            let object = self.stack.pop().unwrap();
            r = r + &object;
        }

        self.stack.push(format!("object compound {}", r));
    }

    fn visit_object_simple(self: &mut Self, _node: &ObjectSimpleNode) {
        let material = self.stack.pop().unwrap();
        let shape = self.stack.pop().unwrap();
        self.stack.push(format!("object simple {} {}", shape, material));
    }

    fn visit_object_transformed(self: &mut Self, _node: &ObjectTransformedNode) {
        let transform = self.stack.pop().unwrap();
        let object = self.stack.pop().unwrap();
        self.stack.push(format!("object transformed {} {}", object, transform));
    }

    fn visit_shape_aabox(self: &mut Self, node: &AABoxShapeNode) {
        self.stack.push(format!("aabox {}", node.extend));
    }

    fn visit_shape_cylinder(self: &mut Self, node: &CylinderShapeNode) {
        self.stack.push(format!("cylinder {} {}", node.radius, node.height));
    }

    fn visit_shape_plane(self: &mut Self, _node: &PlaneShapeNode) {
        self.stack.push("plane".to_string());
    }

    fn visit_shape_rectangle(self: &mut Self, node: &RectangleShapeNode) {
        self.stack.push(format!("rectangle {} {}", node.half_width, node.half_height));
    }

    fn visit_shape_sphere(self: &mut Self, node: &SphereShapeNode) {
        self.stack.push(format!("sphere {}", node.radius));
    }

    fn visit_material_lambertian(self: &mut Self, _node: &LambertianMaterialNode) {
        let texture = self.stack.pop().unwrap();
        self.stack.push(format!("lambertian {}", texture));
    }

    fn visit_texture_color(self: &mut Self, node: &ColorTextureNode) {
        self.stack.push(format!("color {:?}", node.color));
    }

    fn visit_transform(self: &mut Self, node: &TransformNode) {
        let mut r = String::new();
        for _ in 1..=node.steps.len() {
            let object = self.stack.pop().unwrap();
            r = r + &object;
        }

        self.stack.push(format!("transform {{  {} }}", r));
    }

    fn visit_transform_rotate(self: &mut Self, node: &TransformRotateAxisNode) {
        self.stack.push(format!("rotate {:?} {}", node.axis, node.angle));
    }

    fn visit_transform_translate(self: &mut Self, node: &TransformTranslateNode) {
        self.stack.push(format!("translate {}", node.offset));
    }
}

#[cfg(test)]

mod test {
    use crate::loader::Parser;

    #[test]
    fn test_print() {
        let input = "
    scene
      object transformed
        object simple
          sphere 1.0
          lambertian color 0.2 0.8 0.1
        transform {
            translate 0.0 0.0 2.0
            rotate_x 1.5708
        }

      object simple
        rectangle 255.0 123
        lambertian color 0.1 0.8 0.4
    ";

        let mut parser = Parser::new(input);
        let scene_node = parser.parse_scene();
        let mut visitor = super::PrintVisitor::new();
        visitor.visit(&scene_node);
    }
}
