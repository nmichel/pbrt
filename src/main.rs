use pbrt::config::Config;
use pbrt::integrators::{self, Integrator, NormalIntegrator, PathIntegrator};
use pbrt::loader::Loader;
use pbrt::renderers;
use std::{env, process, fs};

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    println!("Redering with configuration settings: {:#?}", &config);

    let integrator: Box<dyn Integrator> = match config.integrator {
        integrators::Type::NORMAL => Box::new(NormalIntegrator::new()),
        integrators::Type::PATH => Box::new(PathIntegrator::new(config.max_depth)),
    };

    let render_function = match config.renderer {
        renderers::Type::ST => renderers::st::render,
        renderers::Type::MT => renderers::mt::render,
    };

    let text = fs::read_to_string(&config.input_filename).expect("Should be able to read file");

    let (scene, camera) = Loader::load_scene(&text, &config);

    render_function(&config, &scene, camera.as_ref(), &*integrator);
}
