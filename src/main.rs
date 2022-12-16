use pbrt::config::Config;
use pbrt::integrators::{self, Integrator, NormalIntegrator, PathIntegrator};
use pbrt::renderers;
use std::{env, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    println!("Redering with configuration settings: {:#?}", &config);

    let (scene, camera) = pbrt::demos::csg_substraction_cube_bowl::build_scene(&config);

    let integrator: Box<dyn Integrator> = match config.integrator {
        integrators::Type::NORMAL => Box::new(NormalIntegrator::new()),
        integrators::Type::PATH => Box::new(PathIntegrator::new(config.max_depth)),
    };

    renderers::st::render(&config, &scene, camera.as_ref(), &*integrator);
}
