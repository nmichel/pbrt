use pbrt::cameras::{Camera, PinHoleCamera};
use pbrt::config::Config;
use pbrt::geom::matrix4::Matrix4;
use pbrt::geom::transform::Transform;
use pbrt::geom::vector2::Vector2u;
use pbrt::geom::vector3::Vector3f;
use pbrt::integrators::{self, Integrator, NormalIntegrator, PathIntegrator};
use pbrt::materials::*;
use pbrt::objects::{Compound, Object, Simple, Transformed};
use pbrt::scene::Scene;
use pbrt::shapes::{Cylinder, Sphere};
use pbrt::spectrum::Spectrum;
use pbrt::textures::CheckerBoard;
use pbrt::{colors, renderers};
use std::sync::Arc;
use std::{env, f64, process};

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
        &Vector3f::new(0.0, 1.0, 3.5),
        &Vector3f::new(0.0, 0.0, 0.0),
        &Vector3f::new(0.0, 1.0, 0.0),
    );

    let camera = PinHoleCamera::new(&resolution, fov, near, far, cam_to_world);

    // Scene
    //
    let mut scene = Scene::new();

    let cylinder = Arc::new(Simple::new(
        Arc::new(Cylinder::new(0.3, 0.7)),
        Arc::new(Lambertian::new(Arc::new(CheckerBoard::new(colors::WHITE, colors::CORAL, 10.0)))),
    ));

    let mut cylinders = Vec::new();
    let count = 10;
    let step = f64::consts::PI * 2.0 / count as f64;
    let radius = 1.5;
    for i in 0..count {
        let x = (step * i as f64).cos() * radius;
        let z = (step * i as f64).sin() * radius;

        cylinders.push(Arc::new(Transformed::new(
            cylinder.clone(),
            Box::new(Transform::translation(Vector3f::new(x, 0.0, z))),
        )) as Arc<dyn Object>);
    }
    let cylinder_group = Arc::new(Compound::new(&cylinders));

    scene
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                Arc::new(Sphere::new(1000.0)),
                Arc::new(Lambertian::new(Arc::new(CheckerBoard::new(
                    Spectrum::new(0.2, 0.3, 0.1),
                    Spectrum::new(0.9, 0.9, 0.9),
                    2000.0,
                )))),
            )),
            Box::new(Transform::translation(Vector3f::new(0.0, -1001.0, 0.0))),
        )))
        .add_object(Arc::new(Transformed::new(
            cylinder_group.clone(),
            Box::new(
                Transform::translation(Vector3f::new(0.0, 2.0, 0.0))
                    * Transform::rotation_y(f64::consts::FRAC_PI_6)
                    * Transform::rotation_x(f64::consts::FRAC_PI_4),
            ),
        )));
    scene.add_object(cylinder_group);

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
