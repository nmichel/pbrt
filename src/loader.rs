mod ast;
mod mesh_loader;
mod parser;
mod ply;
mod visitors;

pub use mesh_loader::load_ply_mesh;

use std::sync::Arc;

use self::parser::Parser;
use self::visitors::SceneBuilderVisitor;
use crate::cameras::Camera;
use crate::colors;
use crate::config::Config;
use crate::geom::transform::Transform;
use crate::geom::vector3::Vector3;
use crate::lights::PointLight;
use crate::scene::Scene;

pub struct Loader {}

impl Loader {
    pub fn load_scene(input: &str, config: &Config) -> (Scene, Box<dyn Camera>) {
        let scene = Parser::parse(input);
        let mut visitor = SceneBuilderVisitor::new(config);
        visitor.visit(&scene);
        visitor.scene.commit();

        // let point_light = PointLight::new(Box::new(Transform::translation(Vector3::new(0.0, 2.0, 0.0))), colors::WHITE * 1.0);
        // visitor.scene.add_light(Arc::new(point_light));
        (visitor.scene, visitor.camera.unwrap())
    }
}
