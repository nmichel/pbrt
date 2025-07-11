mod lexer;

use self::lexer::{Token, Tokenizer};
use super::ast::*;
use crate::geom::vector3::Vector3f;
use crate::spectrum::Spectrum;

pub struct Parser<'a> {
    tokenizer: Tokenizer<'a>,
}

impl<'a> Parser<'a> {
    pub fn parse(input: &str) -> StageNode {
        let mut parser = Parser::new(input);
        parser.parse_stage()
    }

    pub fn new(input: &'a str) -> Self {
        Parser {
            tokenizer: Tokenizer::<'a>::new(input),
        }
    }

    // Stage parsing

    pub fn parse_stage(self: &mut Self) -> StageNode {
        let camera = self.parse_camera();
        let scene = self.parse_scene();
        StageNode::new(scene, camera)
    }

    // Camera parsing

    pub fn parse_camera(self: &mut Self) -> Box<dyn CameraNode> {
        if let Token::KWCamera = self.tokenizer.next_token().expect("camera expected") {
            return match self.tokenizer.next_token().expect("camera type expected") {
                Token::KWPinHole => self.parse_camera_pinhole(),
                Token::KWThinLens => self.parse_camera_thin_lens(),
                _ => panic!("Unknown camera type"),
            };
        }

        panic!("camera expected")
    }

    pub fn parse_camera_pinhole(self: &mut Self) -> Box<dyn CameraNode> {
        if let Token::KWPos = self.tokenizer.next_token().expect("pos expected") {
            let pos = self.parse_vector();
            if let Token::KWLook = self.tokenizer.next_token().expect("look expected") {
                let look = self.parse_vector();
                if let Token::KWUp = self.tokenizer.next_token().expect("up expected") {
                    let up = self.parse_vector();
                    return Box::new(PinHoleCameraNode::new(pos, look, up));
                }
            }
        }

        panic!("camera pin_hole camera")
    }

    pub fn parse_camera_thin_lens(self: &mut Self) -> Box<dyn CameraNode> {
        if let Token::KWPos = self.tokenizer.next_token().expect("pos expected") {
            let pos = self.parse_vector();
            if let Token::KWLook = self.tokenizer.next_token().expect("look expected") {
                let look = self.parse_vector();
                if let Token::KWUp = self.tokenizer.next_token().expect("up expected") {
                    let up = self.parse_vector();
                    if let Token::KWRadius = self.tokenizer.next_token().expect("radius expected") {
                        let radius = self.parse_number();
                        if let Token::KWFocalLength = self.tokenizer.next_token().expect("focal_length expected") {
                            let focal_length = self.parse_number();
                            return Box::new(ThinLensCameraNode::new(pos, look, up, radius, focal_length));
                        }
                    }
                }
            }
        }

        panic!("camera pin_hole camera")
    }

    // Scene parsing

    pub fn parse_scene(self: &mut Self) -> SceneNode {
        match self.tokenizer.next_token() {
            Some(Token::KWScene) => self.parse_scene_block(),
            _ => panic!("Expected scene block"),
        }
    }

    fn parse_scene_block(self: &mut Self) -> SceneNode {
        let mut scene_node = SceneNode::new();
        while let Some(token) = self.tokenizer.next_token() {
            match token {
                Token::KWObject => scene_node.add_object(self.parse_object()),
                _ => panic!("Expected object"),
            }
        }
        scene_node
    }

    // Object parsing

    fn parse_object(self: &mut Self) -> Box<dyn ObjectNode> {
        match self.tokenizer.next_token() {
            Some(Token::KWSimple) => self.parse_object_simple(),
            Some(Token::KWTransformed) => self.parse_object_transformed(),
            Some(Token::KWCompound) => self.parse_object_compound(),
            _ => panic!("Expected simple, transformed or compound"),
        }
    }

    fn parse_object_simple(self: &mut Self) -> Box<dyn ObjectNode> {
        let shape = self.parse_shape();
        let material = self.parse_material();
        Box::new(ObjectSimpleNode::new(shape, material))
    }

    fn parse_object_transformed(self: &mut Self) -> Box<dyn ObjectNode> {
        if Token::KWObject == self.tokenizer.next_token().expect("Expected object") {
            let object = self.parse_object();
            let transform = self.parse_transform();
            return Box::new(ObjectTransformedNode::new(object, transform));
        }
        panic!("Expected object")
    }

    fn parse_object_compound(self: &mut Self) -> Box<dyn ObjectNode> {
        if Token::BraceOpen == self.tokenizer.next_token().expect("Expected {") {
            let mut node: Box<ObjectCompoundNode> = Box::new(ObjectCompoundNode::new());
            while let Some(token) = self.tokenizer.next_token() {
                match token {
                    Token::KWObject => node.add_object(self.parse_object()),
                    Token::BraceClose => return node,
                    _ => panic!("{}", "Expected } or object"),
                }
            }
        }
        panic!("{}", "Expected {")
    }

    // Shape parsing

    fn parse_shape(self: &mut Self) -> Box<dyn ShapeNode> {
        match self.tokenizer.next_token() {
            Some(Token::KWAABox) => self.parse_shape_aabox(),
            Some(Token::KWCSG) => self.parse_shape_csg(),
            Some(Token::KWCylinder) => self.parse_shape_cylinder(),
            Some(Token::KWMesh) => self.parse_shape_mesh(),
            Some(Token::KWPlane) => self.parse_shape_plane(),
            Some(Token::KWRectangle) => self.parse_shape_rectangle(),
            Some(Token::KWSphere) => self.parse_shape_sphere(),
            _ => panic!("Expected sphere"),
        }
    }

    fn parse_shape_aabox(self: &mut Self) -> Box<dyn ShapeNode> {
        let extent = self.parse_vector();
        return Box::new(AABoxShapeNode::new(extent));
    }

    fn parse_shape_csg(self: &mut Self) -> Box<dyn ShapeNode> {
        match self.tokenizer.next_token() {
            Some(Token::KWIntersection) => self.parse_shape_csg_intersection(),
            Some(Token::KWUnion) => self.parse_shape_csg_union(),
            Some(Token::KWSubtraction) => self.parse_shape_csg_substraction(),
            _ => panic!("Expected intersection, union or subtraction"),
        }
    }

    fn parse_shape_csg_intersection(self: &mut Self) -> Box<dyn ShapeNode> {
        let elems = self.parse_shape_csg_elements();
        return Box::new(CSGShapeIntersectionNode::new(elems));
    }

    fn parse_shape_csg_union(self: &mut Self) -> Box<dyn ShapeNode> {
        let elems = self.parse_shape_csg_elements();
        return Box::new(CSGShapeUnionNode::new(elems));
    }

    fn parse_shape_csg_substraction(self: &mut Self) -> Box<dyn ShapeNode> {
        let elems = self.parse_shape_csg_elements();
        return Box::new(CSGShapeSubstractionNode::new(elems));
    }

    fn parse_shape_csg_elements(self: &mut Self) -> Vec<Box<CSGShapeElemNode>> {
        if Token::BraceOpen == self.tokenizer.next_token().expect("Expected {") {
            let mut elems: Vec<Box<CSGShapeElemNode>> = Vec::new();
            while let Some(token) = self.tokenizer.next_token() {
                match token {
                    Token::KWElem => elems.push(self.parse_shape_csg_element()),
                    Token::BraceClose => return elems,
                    _ => panic!("{}", "Expected } or elem"),
                }
            }
        }
        panic!("{}", "Expected {")
    }

    fn parse_shape_csg_element(self: &mut Self) -> Box<CSGShapeElemNode> {
        let shape = self.parse_shape();
        let transform = self.parse_transform();
        Box::new(CSGShapeElemNode::new(shape, transform))
    }

    fn parse_shape_cylinder(self: &mut Self) -> Box<dyn ShapeNode> {
        if let Token::Number(radius) = self.tokenizer.next_token().expect("Expected radius") {
            if let Token::Number(height) = self.tokenizer.next_token().expect("Expected height value") {
                return Box::new(CylinderShapeNode::new(radius, height));
            }
        }
        panic!("Expected cylinder");
    }

    fn parse_shape_mesh(self: &mut Self) -> Box<dyn ShapeNode> {
        if let Token::KWFile = self.tokenizer.next_token().expect("Expected file") {
            if let Token::String(filename) = self.tokenizer.next_token().expect("Expected filename") {
                let mut r = Box::new(MeshShapeNode::new(filename));
                match self.tokenizer.next_token() {
                    Some(Token::KWReverse) => r.set_reverse(true),
                    Some(token) => self.tokenizer.take_back(token),
                    _ => {}
                }
                return r;
            }
        }
        panic!("Expected mesh file");
    }

    fn parse_shape_plane(self: &mut Self) -> Box<dyn ShapeNode> {
        Box::new(PlaneShapeNode::new())
    }

    fn parse_shape_rectangle(self: &mut Self) -> Box<dyn ShapeNode> {
        if let Token::Number(half_width) = self.tokenizer.next_token().expect("Expected half_witdh value") {
            if let Token::Number(half_height) = self.tokenizer.next_token().expect("Expected half_height value") {
                return Box::new(RectangleShapeNode::new(half_width, half_height));
            }
        }
        panic!("Expected radius");
    }

    fn parse_shape_sphere(self: &mut Self) -> Box<dyn ShapeNode> {
        match self.tokenizer.next_token() {
            Some(Token::Number(radius)) => Box::new(SphereShapeNode::new(radius)),
            _ => panic!("Expected radius"),
        }
    }

    // Material parsing

    fn parse_material(self: &mut Self) -> Box<dyn MaterialNode> {
        match self.tokenizer.next_token() {
            Some(Token::KWDielectric) => self.parse_material_dielectric(),
            Some(Token::KWDiffuseLight) => self.parse_material_diffuse_light(),
            Some(Token::KWLambertian) => self.parse_material_lambertian(),
            Some(Token::KWMetal) => self.parse_material_metal(),
            _ => panic!("Expected lambertian, dielectric, metal"),
        }
    }

    fn parse_material_dielectric(self: &mut Self) -> Box<dyn MaterialNode> {
        let index = self.parse_number();
        let albedo = self.parse_texture();
        Box::new(DielectricMaterialNode::new(index, albedo))
    }

    fn parse_material_diffuse_light(self: &mut Self) -> Box<dyn MaterialNode> {
        Box::new(DiffuseLightMaterialNode::new(self.parse_texture()))
    }

    fn parse_material_lambertian(self: &mut Self) -> Box<dyn MaterialNode> {
        Box::new(LambertianMaterialNode::new(self.parse_texture()))
    }

    fn parse_material_metal(self: &mut Self) -> Box<dyn MaterialNode> {
        let fuzz = self.parse_number();
        let albedo = self.parse_texture();
        Box::new(MetalMaterialNode::new(fuzz, albedo))
    }

    // Texture parsing

    fn parse_texture(self: &mut Self) -> Box<dyn TextureNode> {
        match self.tokenizer.next_token() {
            Some(Token::KWColor) => Box::new(ColorTextureNode::new(self.parse_spectrum())),
            Some(Token::KWCheckerboard) => self.parse_texture_checkerboard(),
            _ => panic!("Expected color, checkeboard"),
        }
    }

    fn parse_texture_checkerboard(self: &mut Self) -> Box<dyn TextureNode> {
        let color1 = self.parse_spectrum();
        let color2 = self.parse_spectrum();
        let scale = self.parse_number();
        Box::new(CheckerboardTextureNode::new(color1, color2, scale))
    }

    fn parse_transform(self: &mut Self) -> Box<TransformNode> {
        if Token::KWTransform == self.tokenizer.next_token().expect("Expected transform") {
            if Token::BraceOpen == self.tokenizer.next_token().expect("Expected {") {
                let mut node: Box<TransformNode> = Box::new(TransformNode::new());
                while let Some(token) = self.tokenizer.next_token() {
                    match token {
                        Token::KWTranslate => {
                            let offset = self.parse_vector();
                            let translate_node = TransformTranslateNode::new(offset);
                            node.add_step(Box::new(translate_node));
                        }
                        Token::KWRotateX => {
                            let angle = self.parse_number();
                            let rotate_node = TransformRotateAxisNode::new(angle, Axis::X);
                            node.add_step(Box::new(rotate_node));
                        }
                        Token::KWRotateY => {
                            let angle = self.parse_number();
                            let rotate_node = TransformRotateAxisNode::new(angle, Axis::Y);
                            node.add_step(Box::new(rotate_node));
                        }
                        Token::KWRotateZ => {
                            let angle = self.parse_number();
                            let rotate_node = TransformRotateAxisNode::new(angle, Axis::Z);
                            node.add_step(Box::new(rotate_node));
                        }
                        Token::BraceClose => {
                            return node;
                        }
                        _ => panic!("Expected object"),
                    }
                }
            }
        }
        panic!("Expected transform")
    }

    // spectrum parsing

    fn parse_spectrum(self: &mut Self) -> Spectrum {
        if let Token::Number(red) = self.tokenizer.next_token().expect("Expected red value") {
            if let Token::Number(green) = self.tokenizer.next_token().expect("Expected green value") {
                if let Token::Number(blue) = self.tokenizer.next_token().expect("Expected blue value") {
                    return Spectrum::new(red, green, blue);
                }
            }
        }
        panic!("Expected color")
    }

    // vector parsing

    fn parse_number(self: &mut Self) -> f64 {
        if let Token::Number(x) = self.tokenizer.next_token().expect("Expected number") {
            return x;
        }
        panic!("Expected number")
    }

    fn parse_vector(self: &mut Self) -> Vector3f {
        if let Token::Number(x) = self.tokenizer.next_token().expect("Expected x value") {
            if let Token::Number(y) = self.tokenizer.next_token().expect("Expected y value") {
                if let Token::Number(z) = self.tokenizer.next_token().expect("Expected z value") {
                    return Vector3f::new(x, y, z);
                }
            }
        }
        panic!("Expected vector")
    }
}

#[cfg(test)]
mod test {
    use crate::loader::visitors::PrintVisitor;

    use super::*;

    #[test]
    fn test_parse() {
        let input = "
      scene
        object transformed
          object simple
            csg union {
              elem
                sphere 0.4
                transform {
                  translate -0.5 0 -0.5
                }

              elem
                sphere 0.4
                transform {
                  translate 0 0 -0.5
                }

              elem
                sphere 0.4
                transform {
                  translate 0.5 0 -0.5
                }

              elem
                sphere 0.4
                transform {
                  translate -0.5 0 0
                }

              elem
                sphere 0.4
                transform {
                  translate 0 0 0
                }

              elem
                sphere 0.4
                transform {
                  translate 0.5 0 0
                }

              elem
                sphere 0.4
                transform {
                  translate -0.5 0 0.5
                }

              elem
                sphere 0.4
                transform {
                  translate 0 0 0.5
                }

              elem
                sphere 0.4
                transform {
                  translate 0.5 0 0.5
                }
            }
            lambertian color 1 0.6470588235294118 0
          transform {
            translate 0 0 0
          }
        object transformed
          object simple
            rectangle 3 3
            lambertian color 0.5 0.5 0.5
          transform {
            translate 0 -0.6 0
          }
    ";
        let mut parser = Parser::new(input);
        let scene = parser.parse_scene();
        let mut print_visitor = PrintVisitor::new();
        print_visitor.visit(&scene);
    }
}
