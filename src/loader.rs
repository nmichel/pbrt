mod ast;
mod parser;
mod visitors;

use crate::scene::Scene;

use self::parser::Parser;
use self::visitors::SceneBuilderVisitor;

pub struct Loader {}

impl Loader {
    pub fn load_scene(input: &str) -> Scene {
        let mut parser = Parser::new(input);
        let mut visitor = SceneBuilderVisitor::new();
        let scene = parser.parse_scene();
        visitor.visit(&scene);
        visitor.scene.commit();
        visitor.scene
    }
}
