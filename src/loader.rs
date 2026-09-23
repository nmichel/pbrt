mod ast;
mod mesh_loader;
mod parser;
mod ply;
mod visitors;

pub use mesh_loader::load_ply_mesh;

use self::parser::Parser;
use self::visitors::SceneBuilderVisitor;
use crate::cameras::Camera;
use crate::config::Config;
use crate::scene::Scene;

pub struct Loader {}

impl Loader {
    /// The scene a `.stage` description asks for, and nothing besides.
    ///
    /// Lighting comes from the file like everything else: the loader adds no source of its own,
    /// so a scene is lit by what it declares. It used to add a point light and a sky to every
    /// description, which made a `.stage` file unable to say what it was lit by — and made the
    /// two integrators impossible to compare, `naive` never consulting `Scene::lights` at all.
    pub fn load_scene(input: &str, config: &Config) -> (Scene, Box<dyn Camera>) {
        let scene = Parser::parse(input);

        let mut visitor = SceneBuilderVisitor::new(config);
        visitor.visit(&scene);
        visitor.scene.commit();

        // Not an error: a description is allowed to say this, and an emissive surface the camera
        // looks straight at is still seen. Everything else is black, though, and the reason is a
        // line missing from a file rather than anything the renderer did.
        if visitor.scene.get_light_count() == 0 {
            eprintln!("warning: this scene declares no `light`; only emission the camera sees directly will show, the rest is black");
        }

        (visitor.scene, visitor.camera.unwrap())
    }
}
