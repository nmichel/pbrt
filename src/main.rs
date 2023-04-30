use pbrt::config::Config;
use pbrt::integrators::{self, Integrator, NormalIntegrator, PathIntegrator};
use pbrt::loader::Loader;
use pbrt::parser::{Node, Parser, SceneBuilderVisitor};
use pbrt::renderers;
use std::{env, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    println!("Redering with configuration settings: {:#?}", &config);

    //    let (mut scene, camera) = pbrt::demos::cornel_box::build_scene(&config);
    let (_, camera) = pbrt::demos::test_simple_object::build_scene(&config);
    //    scene.commit();

    let integrator: Box<dyn Integrator> = match config.integrator {
        integrators::Type::NORMAL => Box::new(NormalIntegrator::new()),
        integrators::Type::PATH => Box::new(PathIntegrator::new(config.max_depth)),
    };

    let render_function = match config.renderer {
        renderers::Type::ST => renderers::st::render,
        renderers::Type::MT => renderers::mt::render,
    };

    let input = "
    scene
    # A simple diffuse sphere
    object simple
      sphere 1.0
      lambertian color 0.2 0.8 0.1
    ";

    let scene = Loader::load_scene(input);

    render_function(&config, &scene, camera.as_ref(), &*integrator);
}
