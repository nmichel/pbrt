use pbrt::cameras::{Camera, PinHoleCamera};
use pbrt::config::Config;
use pbrt::geom::matrix4::Matrix4;
use pbrt::geom::transform::Transform;
use pbrt::geom::vector2::Vector2u;
use pbrt::geom::vector3::Vector3f;
use pbrt::integrators::{self, Integrator, NaiveIntegrator, NormalIntegrator, PathIntegrator};
use pbrt::lights::BackgroundInfiniteLight;
use pbrt::materials::*;
use pbrt::objects::{Simple, Transformed};
use pbrt::scene::Scene;
use pbrt::shapes::{AABox, Sphere};
use pbrt::spectrum::Spectrum;
use pbrt::textures::{CheckerBoard, PlainColor, Texture};
use pbrt::{colors, renderers};
use std::sync::Arc;
use std::{env, f64, process};

pub fn build_scene(config: &Config) -> (Scene, Box<dyn Camera>) {
    // Sphere refléchissante
    // Cube transparent
    // Sphere jaune
    // Sol sphere damier

    // Camera
    //
    let image_width = config.output_width as u32;
    let image_height = config.output_height as u32;
    let fov = config.fov_deg * f64::consts::PI / 180.0;
    let near = config.near;
    let far = config.far;

    let resolution = Vector2u::new(image_width, image_height);
    let cam_to_world = Matrix4::look_at(
        &Vector3f::new(0.7, 0.6, 0.5),
        &Vector3f::new(0.0, 0.0, -1.0),
        &Vector3f::new(0.0, 1.0, 0.0),
    );

    let camera = PinHoleCamera::new(&resolution, fov, near, far, cam_to_world);

    // Scene
    //
    let text_check_red: Arc<dyn Texture> = Arc::new(CheckerBoard::new(Spectrum::new(0.65, 0.0, 0.0), Spectrum::new(0.65, 0.65, 0.65), 1000.0));
    let material_wall: Arc<dyn Material> = Arc::new(Lambertian::new(Arc::clone(&text_check_red)));
    let material_lambertian: Arc<dyn Material> = Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::ORANGE))));
    let material_dielectric: Arc<dyn Material> = Arc::new(Dielectric::new(RefractionIndices::WATER, Arc::new(PlainColor::new(colors::WHITE))));
    let material_metal_white: Arc<dyn Material> = Arc::new(Metal::new(0.0, Arc::new(PlainColor::new(colors::WHITE))));

    let mut scene = Scene::new();
    scene
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(Arc::new(Sphere::new(100.0)), Arc::clone(&material_wall))),
            Box::new(Transform::translation(Vector3f::new(0.0, -100.5, -1.0))),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(Arc::new(Sphere::new(0.5)), Arc::clone(&material_lambertian))),
            Box::new(Transform::translation(Vector3f::new(-1.0, 0.0, -1.0))),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                Arc::new(AABox::new(&Vector3f::new(0.8, 0.8, 0.8))),
                Arc::clone(&material_dielectric),
            )),
            Box::new(Transform::translation(Vector3f::new(0.0, 0.0, -0.6))),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(Arc::new(Sphere::new(0.5)), Arc::clone(&material_metal_white))),
            Box::new(Transform::translation(Vector3f::new(1.0, 0.0, -1.0))),
        )));

    // Nothing in this scene emits, so without a light the `PATH` integrator returns a uniformly
    // black image. The sky of "Ray Tracing in One Weekend" stands in; `examples/csg_bowl.rs`
    // derives why it is needed and what its uniform-sphere sampling costs.
    scene.add_light(Arc::new(BackgroundInfiniteLight::new(colors::WHITE, Spectrum::new(0.5, 0.7, 1.0))));

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
        integrators::Type::NAIVE => Box::new(NaiveIntegrator::new(config.max_depth)),
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
