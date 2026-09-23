use super::super::ast::*;
use super::super::ply::{read_ply_file, ElementDesc, ElementProps, PlyEventObserver, PropertyValue};
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

    /// What the visited description prints as.
    ///
    /// Every `visit_*` leaves its own text on the stack and consumes its children's, so a finished
    /// traversal leaves exactly one entry: the whole description. `visit_stage` prints it; a test
    /// reads it here, which is the only way to hold this visitor to its promise — that the order
    /// in which the traversal hands it its pieces is its own business, and the text does not move
    /// with it.
    pub fn rendered(&self) -> &str {
        self.stack.last().map_or("", String::as_str)
    }
}

impl Visitor for PrintVisitor {
    fn visit_stage(self: &mut Self, _node: &StageNode) {
        let scene = self.stack.pop().unwrap();
        let camera = self.stack.pop().unwrap();
        println!("stage {} {}", scene, camera);
    }

    /// Lights first, then objects — the order the scene node holds them in, not the order they
    /// were written in. A description mixing the two therefore comes back with its lights
    /// gathered in front ([`SceneNode`]).
    fn visit_scene(self: &mut Self, node: &SceneNode) {
        let mut r = String::new();
        for _ in 1..=node.objects.len() {
            let object = self.stack.pop().unwrap();
            r = object + &r;
        }

        for _ in 1..=node.lights.len() {
            let light = self.stack.pop().unwrap();
            r = light + &r;
        }

        self.stack.push(format!("scene {}", r));
    }

    fn visit_camera_pin_hole(self: &mut Self, node: &PinHoleCameraNode) {
        self.stack.push(format!("camera pin hole {} {} {}", node.pos, node.look, node.up));
    }

    fn visit_camera_thin_lens(self: &mut Self, node: &ThinLensCameraNode) {
        self.stack.push(format!(
            "camera thin lens {} {} {} {} {}",
            node.pos, node.look, node.up, node.radius, node.focal_length
        ));
    }

    fn visit_object_compound(self: &mut Self, node: &ObjectCompoundNode) {
        let mut r = String::new();
        for _ in 1..=node.objects.len() {
            let object = self.stack.pop().unwrap();
            r = object + &r;
        }

        self.stack.push(format!("object compound {}", r));
    }

    fn visit_object_simple(self: &mut Self, _node: &ObjectSimpleNode) {
        let material = self.stack.pop().unwrap();
        let shape = self.stack.pop().unwrap();
        self.stack.push(format!("object simple {} {}", shape, material));
    }

    /// Nothing to do on the way in: this visitor carries no context, only a stack of strings, and
    /// the transformation is already on it — it is read on the way out.
    fn enter_object_transformed(self: &mut Self, _node: &ObjectTransformedNode) {}

    /// The object sits on top of the transformation, the transformation having been printed first.
    ///
    /// The two pops are therefore in the opposite order to the printed text, which is unchanged:
    /// `.stage` in, the same `.stage` out.
    fn leave_object_transformed(self: &mut Self, _node: &ObjectTransformedNode) {
        let object = self.stack.pop().unwrap();
        let transform = self.stack.pop().unwrap();
        self.stack.push(format!("object transformed {} {}", object, transform));
    }

    /// The placement was visited first, so it is already on the stack when the light is
    /// announced — the same shape as a `csg elem`.
    fn visit_light_point(self: &mut Self, node: &PointLightNode) {
        let transform = self.stack.pop().unwrap();
        self.stack.push(format!("light point color {:?} {}", node.intensity, transform));
    }

    fn visit_light_uniform_infinite(self: &mut Self, node: &UniformInfiniteLightNode) {
        self.stack.push(format!("light uniform_infinite color {:?}", node.radiance));
    }

    fn visit_light_background_infinite(self: &mut Self, node: &BackgroundInfiniteLightNode) {
        self.stack
            .push(format!("light background_infinite color {:?} color {:?}", node.bottom, node.top));
    }

    fn visit_shape_aabox(self: &mut Self, node: &AABoxShapeNode) {
        self.stack.push(format!("aabox {}", node.extend));
    }

    fn visit_shape_csg_elem(self: &mut Self, _node: &CSGShapeElemNode) {
        let transform = self.stack.pop().unwrap();
        let elem = self.stack.pop().unwrap();
        self.stack.push(format!("elem {} {}", elem, transform));
    }

    fn visit_shape_csg_intersection(self: &mut Self, node: &CSGShapeIntersectionNode) {
        let mut r = String::new();
        for _ in 1..=node.elems.len() {
            let object = self.stack.pop().unwrap();
            r = object + &r;
        }

        self.stack.push(format!("csg intersection {{  {} }}", r));
    }

    fn visit_shape_csg_substraction(self: &mut Self, node: &CSGShapeSubstractionNode) {
        let mut r = String::new();
        for _ in 1..=node.elems.len() {
            let object = self.stack.pop().unwrap();
            r = object + &r;
        }

        self.stack.push(format!("csg substraction {{  {} }}", r));
    }

    fn visit_shape_csg_union(self: &mut Self, node: &CSGShapeUnionNode) {
        let mut r = String::new();
        for _ in 1..=node.elems.len() {
            let object = self.stack.pop().unwrap();
            r = object + &r;
        }

        self.stack.push(format!("csg union {{  {} }}", r));
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

    fn visit_shape_mesh(self: &mut Self, node: &MeshShapeNode) {
        struct PlyObserver {}

        impl PlyEventObserver for PlyObserver {
            fn on_vertex_event(self: &mut Self, props: &ElementProps, value: PropertyValue) {
                println!("Vertex Event for Property: {:?} with Value: {:?}", props, value);
            }

            fn on_face_event(self: &mut Self, props: &ElementProps, value: PropertyValue) {
                println!("Face Event for Property: {:?} with Value: {:?}", props, value);
            }
        }

        read_ply_file(&node.filename, &mut PlyObserver {}).unwrap();

        self.stack.push(format!("mesh {}", node.filename));
    }

    fn visit_material_dielectric(self: &mut Self, node: &DielectricMaterialNode) {
        let texture = self.stack.pop().unwrap();
        self.stack.push(format!("dielectric {} {}", node.index, texture));
    }

    fn visit_material_diffuse_light(self: &mut Self, _node: &DiffuseLightMaterialNode) {
        let texture = self.stack.pop().unwrap();
        self.stack.push(format!("diffuse_light {}", texture));
    }

    fn visit_material_lambertian(self: &mut Self, _node: &LambertianMaterialNode) {
        let texture = self.stack.pop().unwrap();
        self.stack.push(format!("lambertian {}", texture));
    }

    fn visit_material_metal(self: &mut Self, node: &MetalMaterialNode) {
        let texture = self.stack.pop().unwrap();
        self.stack.push(format!("metal {} {}", node.fuzz, texture));
    }

    fn visit_texture_checkerboard(self: &mut Self, node: &CheckerboardTextureNode) {
        self.stack
            .push(format!("checkerboard {:?} {:?} {}", node.color1, node.color2, node.scale));
    }

    fn visit_texture_color(self: &mut Self, node: &ColorTextureNode) {
        self.stack.push(format!("color {:?}", node.color));
    }

    fn visit_transform(self: &mut Self, node: &TransformNode) {
        let mut r = String::new();
        for _ in 1..=node.steps.len() {
            let object = self.stack.pop().unwrap();
            r = object + &r;
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

      object simple
        mesh file \"./test_files/bun_zipper.ply\"
        lambertian color 0.1 0.8 0.4
    ";

        let mut parser = Parser::new(input);
        let scene_node = parser.parse_scene();
        let mut visitor = super::PrintVisitor::new();
        visitor.visit(&scene_node);
    }

    /// A placement is visited *before* the object it places, so the two are popped in the opposite
    /// order to the text they are printed in. This is what says the text stayed put.
    #[test]
    fn test_a_placement_prints_after_the_object_it_places() {
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
    ";

        let mut parser = Parser::new(input);
        let scene_node = parser.parse_scene();
        let mut visitor = super::PrintVisitor::new();
        visitor.visit(&scene_node);

        assert_eq!(
            visitor.rendered(),
            "scene object transformed object simple sphere 1 lambertian color Spectrum { spectrum: [0.2, 0.8, 0.1] } transform {  translate [0 0 \
             2]rotate X 1.5708 }"
        );
    }

    /// The three lights with no geometry, each with the numbers it takes and no others.
    ///
    /// A point light carries a placement and the two infinite ones do not, which is what this
    /// text says: there is no position to give something that is nowhere in particular.
    #[test]
    fn test_the_three_lights_print_what_they_were_given() {
        let input = "
    scene
      light point
        color 15.0 15.0 15.0
        transform {
            translate 0.0 2.0 1.0
        }

      light uniform_infinite
        color 0.5 0.5 0.5

      light background_infinite
        color 1.0 1.0 1.0
        color 0.5 0.7 1.0
    ";

        let mut parser = Parser::new(input);
        let scene_node = parser.parse_scene();
        let mut visitor = super::PrintVisitor::new();
        visitor.visit(&scene_node);

        assert_eq!(
            visitor.rendered(),
            "scene light point color Spectrum { spectrum: [15.0, 15.0, 15.0] } transform {  translate [0 2 1] }light uniform_infinite color \
             Spectrum { spectrum: [0.5, 0.5, 0.5] }light background_infinite color Spectrum { spectrum: [1.0, 1.0, 1.0] } color Spectrum { \
             spectrum: [0.5, 0.7, 1.0] }"
        );
    }

    /// Lights come out in front of objects whatever order the file mixed them in — the normal
    /// form the two sibling lists of [`SceneNode`](crate::loader::ast::SceneNode) impose.
    #[test]
    fn test_a_light_prints_ahead_of_the_objects_it_was_written_among() {
        let input = "
    scene
      object simple
        sphere 1.0
        lambertian color 0.2 0.8 0.1

      light uniform_infinite
        color 0.5 0.5 0.5
    ";

        let mut parser = Parser::new(input);
        let scene_node = parser.parse_scene();
        let mut visitor = super::PrintVisitor::new();
        visitor.visit(&scene_node);

        assert_eq!(
            visitor.rendered(),
            "scene light uniform_infinite color Spectrum { spectrum: [0.5, 0.5, 0.5] }object simple sphere 1 lambertian color Spectrum { spectrum: \
             [0.2, 0.8, 0.1] }"
        );
    }
}
