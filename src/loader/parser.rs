mod lexer;

use self::lexer::{Token, Tokenizer};
use super::ast::*;
use crate::geom::vector3::Vector3f;
use crate::spectrum::Spectrum;

pub struct Parser<'a> {
    tokenizer: Tokenizer<'a>,
}

impl<'a> Parser<'a> {
    pub fn parse(input: &str) -> SceneNode {
        let mut parser = Parser::new(input);
        parser.parse_scene()
    }

    pub fn new(input: &'a str) -> Self {
        Parser {
            tokenizer: Tokenizer::<'a>::new(input),
        }
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
                    Token::KWObject => {
                        let object = self.parse_object();
                        node.add_object(object);
                    }

                    Token::BraceClose => {
                        return node;
                    }

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
            Some(Token::KWCylinder) => self.parse_shape_cylinder(),
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

    fn parse_shape_cylinder(self: &mut Self) -> Box<dyn ShapeNode> {
        if let Token::Number(radius) = self.tokenizer.next_token().expect("Expected radius") {
            if let Token::Number(height) = self.tokenizer.next_token().expect("Expected height value") {
                return Box::new(CylinderShapeNode::new(radius, height));
            }
        }
        panic!("Expected cylinder");
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
            Some(Token::KWLambertian) => self.parse_material_lambertian(),
            _ => panic!("Expected lambertian"),
        }
    }

    fn parse_material_lambertian(self: &mut Self) -> Box<dyn MaterialNode> {
        Box::new(LambertianMaterialNode::new(self.parse_texture()))
    }

    // Texture parsing

    fn parse_texture(self: &mut Self) -> Box<dyn TextureNode> {
        match self.tokenizer.next_token() {
            Some(Token::KWColor) => Box::new(ColorTextureNode::new(self.parse_spectrum())),
            _ => panic!("Expected plain color"),
        }
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
        # A simple diffuse sphere
        object simple
          sphere 1.0
          lambertian color 0.2 0.5 0.9

        # a transformed diffuse sphere
        object transformed
          object simple
            sphere 1.0
            lambertian color 0.2 0.8 0.1
          transform {
            translate 0.0 0.0 2.0
            rotate_x 0.78
          }

        # a compound object
        object compound {
          object transformed
            object simple
              sphere 1.0
              lambertian color 0.2 0.8 0.1
            transform {
              translate 0.0 0.0 2.0
            }
          object transformed
            object simple
              plane
              lambertian color 0.7 0.2 0.1
            transform {
              translate 0.0 0.0 -0.5
            }
        }
      object simple
        cylinder 0.3 1.0
        lambertian color 0.2 0.32 0.5

      object simple
        aabox 0.3 1.0 2.0
        lambertian color 0.2 0.32 0.5


      ";
        let mut parser = Parser::new(input);
        let scene = parser.parse_scene();
        let mut print_visitor = PrintVisitor::new();
        print_visitor.visit(&scene);
    }
}
