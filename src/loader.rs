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
use crate::lights::{BackgroundInfiniteLight, PointLight, UniformInfiniteLight};
use crate::materials::{Lambertian, Metal};
use crate::objects::Simple;
use crate::scene::Scene;
use crate::shapes::{Triangle, TriangleMesh};
use crate::spectrum::Spectrum;
use crate::textures::{CheckerBoard, PlainColor};

pub struct Loader {}

impl Loader {
    pub fn load_scene(input: &str, config: &Config) -> (Scene, Box<dyn Camera>) {
        let scene = Parser::parse(input);

        let mut visitor = SceneBuilderVisitor::new(config);
        visitor.visit(&scene);
        visitor.scene.commit();

        visitor.scene.add_light(Arc::new(PointLight::new(
            Box::new(Transform::translation(Vector3::new(0.0, 2.0, 1.0))),
            colors::WHITE * 15.0,
        )));
        visitor
            .scene
            .add_light(Arc::new(BackgroundInfiniteLight::new(colors::WHITE, Spectrum::new(0.5, 0.7, 1.0))));
        // visitor.scene.add_light(Arc::new(UniformInfiniteLight::new(colors::WHITE * 0.5)));

        (visitor.scene, visitor.camera.unwrap())
    }
}
