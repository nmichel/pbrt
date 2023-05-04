mod ast;
mod parser;
mod visitors;

use crate::scene::Scene;

use self::parser::Parser;
use self::visitors::SceneBuilderVisitor;

pub struct Loader {}

impl Loader {
    pub fn load_scene(input: &str) -> Scene {
        let scene = Parser::parse(input);
        let mut visitor = SceneBuilderVisitor::new();
        visitor.visit(&scene);
        visitor.scene.commit();
        visitor.scene
    }
}
