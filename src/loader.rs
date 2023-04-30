use crate::parser::{Node, Parser, SceneBuilderVisitor};
use crate::scene::Scene;

pub struct Loader {}

impl Loader {
    pub fn load_scene(input: &str) -> Scene {
        let mut parser = Parser::new(input);
        let mut visitor = SceneBuilderVisitor::new();
        parser.parse_scene().visit(&mut visitor);
        visitor.scene.commit();
        visitor.scene
    }
}
