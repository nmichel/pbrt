use pbrt::config::Config;
use pbrt::integrators::{NormalIntegrator, PathIntegrator};
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

    let (scene, camera) = pbrt::demos::csg_union_spheres::build_scene(&config);

    let integrator = PathIntegrator::new(config.max_depth);
    // let integrator = NormalIntegrator::new();

    renderers::mt::render(&config, &scene, camera.as_ref(), &integrator);
}
