use crate::spectrum::Spectrum;

use super::ast::{
    ColorTextureNode, LambertianMaterialNode, MaterialNode, Node, ObjectNode, ObjectSimpleNode, SceneNode, ShapeNode, SphereShapeNode, TextureNode,
};
use super::{Token, Tokenizer};

pub struct Parser<'a> {
    tokenizer: Tokenizer<'a>,
}

impl<'a> Parser<'a> {
    pub fn parse(input: &str) -> () {
        let mut parser = Parser::new(input);
        parser.parse_scene();
    }

    fn new(input: &'a str) -> Self {
        Parser {
            tokenizer: Tokenizer::<'a>::new(input),
        }
    }

    // Scene parsing

    fn parse_scene(self: &mut Self) -> SceneNode {
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
            _ => panic!("Expected simple"),
        }
    }

    fn parse_object_simple(self: &mut Self) -> Box<dyn ObjectNode> {
        let shape = self.parse_shape();
        let material = self.parse_material();
        Box::new(ObjectSimpleNode::new(shape, material))
    }

    // Shape parsing

    fn parse_shape(self: &mut Self) -> Box<dyn ShapeNode> {
        match self.tokenizer.next_token() {
            Some(Token::KWSphere) => self.parse_shape_sphere(),
            _ => panic!("Expected sphere"),
        }
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
}

#[cfg(test)]
mod test {
    use crate::parser::ast::{PrintVisitor, SceneBuilderVisitor};

    use super::*;

    #[test]
    fn test_parse() {
        let input = "
      scene
        # A simple diffuse sphere
        object simple
          sphere 1.0
          lambertian color 0.2 0.5 0.9

        # Another simple diffuse sphere
        object simple
          sphere 2.0
          lambertian color 0.3 0.6 0.8
    ";
        let mut parser = Parser::new(input);
        let scene = parser.parse_scene();
        let mut print_visitor = PrintVisitor::new();
        scene.visit(&mut print_visitor);
    }

    #[test]
    fn test_scenebuilder_visitor() {
        let input = "
      scene
        # A simple diffuse sphere
        object simple
          sphere 1.0
          lambertian color 0.2 0.5 0.9

        # Another simple diffuse sphere
        object simple
          sphere 2.0
          lambertian color 0.3 0.6 0.8
      ";

        let mut parser = Parser::new(input);
        let scene = parser.parse_scene();
        let mut build_visitor = SceneBuilderVisitor::new();
        scene.visit(&mut build_visitor);
    }
}
