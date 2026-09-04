use pbrt::config::{self, Config};
use pbrt::integrators::{self, Integrator, NaiveIntegrator, NormalIntegrator, PathIntegrator};
use pbrt::loader::Loader;
use pbrt::renderers;
use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();

    // Asking for the option list is not a render, so it is answered before a configuration is
    // built — and answered even when the rest of the line would be refused.
    if config::help_requested(&args) {
        println!("{}", config::usage());
        return;
    }

    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("{}, try {} for the option list", err, config::HELP_OPTION);
        process::exit(1);
    });

    println!("{}", &config);

    let integrator: Box<dyn Integrator> = match config.integrator {
        integrators::Type::NAIVE => Box::new(NaiveIntegrator::new(config.max_depth)),
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
