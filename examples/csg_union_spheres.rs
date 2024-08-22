use pbrt::cameras::{Camera, PinHoleCamera};
use pbrt::config::Config;
use pbrt::geom::matrix4::Matrix4;
use pbrt::geom::transform::Transform;
use pbrt::geom::vector2::Vector2u;
use pbrt::geom::vector3::Vector3f;
use pbrt::integrators::{self, Integrator, NormalIntegrator, PathIntegrator};
use pbrt::materials::*;
use pbrt::objects::{Simple, Transformed};
use pbrt::scene::Scene;
use pbrt::shapes::{csg, Rectangle, Sphere};
use pbrt::spectrum::Spectrum;
use pbrt::textures::{CheckerBoard, PlainColor};
use pbrt::{colors, renderers};
use std::sync::Arc;
use std::{env, f64, process};

pub fn build_scene(config: &Config) -> (Scene, Box<dyn Camera>) {
    // CSG Union of 9 orange spheres

    // Camera
    //
    let image_width = config.output_width as u32;
    let image_height = config.output_height as u32;
    let fov = config.fov_deg * f64::consts::PI / 180.0;
    let near = config.near;
    let far = config.far;

    let resolution = Vector2u::new(image_width, image_height);
    let cam_to_world = Matrix4::look_at(
        &Vector3f::new(1.0, 1.3, 1.8),
        &Vector3f::new(0.0, -1.0, 0.0),
        &Vector3f::new(0.0, 1.0, 0.0),
    );

    let camera = PinHoleCamera::new(&resolution, fov, near, far, cam_to_world);

    // Scene
    //
    let mut scene = Scene::new();
    let elements = vec![
        Box::new(csg::Elem {
            shape: Arc::new(Sphere::new(0.4)),
            transform: Box::new(Transform::translation(Vector3f::new(-0.5, 0.0, -0.5))),
        }),
        Box::new(csg::Elem {
            shape: Arc::new(Sphere::new(0.4)),
            transform: Box::new(Transform::translation(Vector3f::new(0.0, 0.0, -0.5))),
        }),
        Box::new(csg::Elem {
            shape: Arc::new(Sphere::new(0.4)),
            transform: Box::new(Transform::translation(Vector3f::new(0.5, 0.0, -0.5))),
        }),
        Box::new(csg::Elem {
            shape: Arc::new(Sphere::new(0.4)),
            transform: Box::new(Transform::translation(Vector3f::new(-0.5, 0.0, 0.0))),
        }),
        Box::new(csg::Elem {
            shape: Arc::new(Sphere::new(0.4)),
            transform: Box::new(Transform::translation(Vector3f::new(0.0, 0.0, 0.0))),
        }),
        Box::new(csg::Elem {
            shape: Arc::new(Sphere::new(0.4)),
            transform: Box::new(Transform::translation(Vector3f::new(0.5, 0.0, 0.0))),
        }),
        Box::new(csg::Elem {
            shape: Arc::new(Sphere::new(0.4)),
            transform: Box::new(Transform::translation(Vector3f::new(-0.5, 0.0, 0.5))),
        }),
        Box::new(csg::Elem {
            shape: Arc::new(Sphere::new(0.4)),
            transform: Box::new(Transform::translation(Vector3f::new(0.0, 0.0, 0.5))),
        }),
        Box::new(csg::Elem {
            shape: Arc::new(Sphere::new(0.4)),
            transform: Box::new(Transform::translation(Vector3f::new(0.5, 0.0, 0.5))),
        }),
    ];

    scene
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                Arc::new(csg::Union::new(elements)),
                Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::ORANGE)))),
            )),
            Box::new(Transform::translation(Vector3f::new(0.0, 0.0, 0.0))),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                Arc::new(Rectangle::new(3.0, 3.0)),
                Arc::new(Lambertian::new(Arc::new(CheckerBoard::new(
                    Spectrum::new(0.65, 0.0, 0.0),
                    Spectrum::new(0.65, 0.65, 0.65),
                    2.0,
                )))),
            )),
            Box::new(Transform::translation(Vector3f::new(0.0, -0.6, 0.0))),
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
