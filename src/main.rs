use pbrt::config::Config;
use pbrt::integrators::{Integrator, NormalIntegrator, PathIntegrator, self};
use pbrt::renderers;
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    println!("Redering with configuration settings: {:#?}", &config);

    let (scene, camera) = pbrt::demos::csg_substraction_cube_sphere::build_scene(&config);

    let integrator: Box<dyn Integrator> = 
        match config.integrator {
            integrators::Type::NORMAL => {
                Box::new(NormalIntegrator::new())
            }
            integrators::Type::PATH => {
                Box::new(PathIntegrator::new(config.max_depth))
            }
        };

    renderers::st::render(&config, &scene, camera.as_ref(), &*integrator);
}
