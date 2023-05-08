use pbrt::cameras::{Camera, PinHoleCamera};
use pbrt::config::Config;
use pbrt::geom::matrix4::Matrix4;
use pbrt::geom::transform::Transform;
use pbrt::geom::vector2::Vector2u;
use pbrt::geom::vector3::Vector3f;
use pbrt::integrators::{Integrator, NormalIntegrator, PathIntegrator, self};
use pbrt::materials::*;
use pbrt::objects::{Simple, Transformed};
use pbrt::scene::Scene;
use pbrt::shapes::{Sphere};
use pbrt::spectrum::Spectrum;
use pbrt::textures::{CheckerBoard};
use pbrt::{renderers};
use std::f64;
use std::f64::consts::FRAC_PI_3;
use std::sync::Arc;
use std::{env, process};

pub fn build_scene(config: &Config) -> (Scene, Box<dyn Camera>) {
    // Camera
    //
    let image_width = config.output_width as u32;
    let image_height = config.output_height as u32;
    let fov = config.fov_deg * f64::consts::PI / 180.0;
    let near = config.near;
    let far = config.far;

    let resolution = Vector2u::new(image_width, image_height);
    let cam_to_world = Matrix4::look_at(
        &Vector3f::new(0.0, 0.0, 2.5),
        &Vector3f::new(0.0, 0.0, 0.0),
        &Vector3f::new(0.0, 1.0, 0.0),
    );

    let camera = PinHoleCamera::new(&resolution, fov, near, far, cam_to_world);

    // Scene
    //
    let mut scene = Scene::new();

    scene.add_object(Arc::new(Transformed::new(
        Arc::new(Simple::new(
            Arc::new(Sphere::new(1.0)),
            Arc::new(Lambertian::new(Arc::new(CheckerBoard::new(
                Spectrum::new(0.2, 0.3, 0.1),
                Spectrum::new(0.9, 0.9, 0.9),
                10.0,
            )))),
        )),
        Box::new(Transform::translation(Vector3f::new(0.0, 1.0, 0.0)) * Transform::rotation_x(FRAC_PI_3)),
    )));

    (scene, Box::new(camera))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1)
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

    let (mut scene, camera) = build_scene(&config);
    scene.commit();

    render_function(&config, &scene, camera.as_ref(), &*integrator);
}
