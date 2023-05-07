mod ast;
mod parser;
mod visitors;

use crate::cameras::Camera;
use crate::config::Config;
use crate::scene::Scene;

use self::parser::Parser;
use self::visitors::SceneBuilderVisitor;

pub struct Loader {}

impl Loader {
    pub fn load_scene(input: &str, config: &Config) -> (Scene, Box<dyn Camera>) {
        let scene = Parser::parse(input);
        let mut visitor = SceneBuilderVisitor::new(config);
        visitor.visit(&scene);
        visitor.scene.commit();
        (visitor.scene, visitor.camera.unwrap())
    }
}
