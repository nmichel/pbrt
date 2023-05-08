// Cornel box (Ray Training The Next Week)

use pbrt::cameras::{Camera, PinHoleCamera};
use pbrt::config::Config;
use pbrt::geom::matrix4::Matrix4;
use pbrt::geom::transform::Transform;
use pbrt::geom::vector2::Vector2u;
use pbrt::geom::vector3::Vector3f;
use pbrt::integrators::{self, Integrator, *};
use pbrt::materials::{DiffuseLight, Lambertian};
use pbrt::objects::{Simple, Transformed};
use pbrt::scene::Scene;
use pbrt::shapes::{AABox, Rectangle};
use pbrt::spectrum::Spectrum;
use pbrt::textures::PlainColor;
use pbrt::{colors, renderers};
use std::f64::consts::{FRAC_PI_2, PI};
use std::sync::Arc;
use std::{env, process};

pub fn build_scene(config: &Config) -> (Scene, Box<dyn Camera>) {
    // Sphere refléchissante
    // Cube transparent
    // Sphere jaune
    // Sol sphere damier

    // Camera
    //
    let image_width = config.output_width as u32;
    let image_height = config.output_height as u32;
    // let fov = config.fov_deg * f64::consts::PI / 180.0;
    let fov = 60.0 * std::f64::consts::PI / 180.0;
    let near = config.near;
    let far = config.far;

    let resolution = Vector2u::new(image_width, image_height);
    let lookfrom: Vector3f = Vector3f::new(0.0, 0.0, 800.0);
    let lookat: Vector3f = Vector3f::new(0.0, 0.0, 0.0);
    let vup: Vector3f = Vector3f::new(0.0, 1.0, 0.0);
    let cam_to_world = Matrix4::look_at(&lookfrom, &lookat, &vup);

    let camera = PinHoleCamera::new(&resolution, fov, near, far, cam_to_world);

    // Scene
    //
    let mut scene = Scene::new();
    scene
        // Ceiling
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                Arc::new(Rectangle::new(555.0, 555.0)),
                Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::WHITE)))),
            )),
            Box::new(Transform::translation(Vector3f::new(0.0, 277.0, 0.0)) * Transform::rotation_x(-PI)),
        )))
        // Floor
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                Arc::new(Rectangle::new(555.0, 555.0)),
                Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::WHITE)))),
            )),
            Box::new(Transform::translation(Vector3f::new(0.0, -277.0, 0.0))),
        )))
        // Right
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                Arc::new(Rectangle::new(555.0, 555.0)),
                Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::RED)))),
            )),
            Box::new(Transform::translation(Vector3f::new(-277.0, 0.0, 0.0)) * Transform::rotation_z(-FRAC_PI_2)),
        )))
        // Left
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                Arc::new(Rectangle::new(555.0, 555.0)),
                Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::GREEN)))),
            )),
            Box::new(Transform::translation(Vector3f::new(277.0, 0.0, 0.0)) * Transform::rotation_z(FRAC_PI_2)),
        )))
        // Back
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                Arc::new(Rectangle::new(555.0, 555.0)),
                Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::WHITE)))),
            )),
            Box::new(Transform::translation(Vector3f::new(0.0, 0.0, -277.0)) * Transform::rotation_x(FRAC_PI_2)),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                Arc::new(AABox::new(&Vector3f::new(165.0, 330.0, 165.0))),
                Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::WHITE)))),
            )),
            Box::new(Transform::translation(Vector3f::new(80.0, -150.0, -100.0)) * Transform::rotation_y(-15.0 * PI / 180.0)),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                Arc::new(AABox::new(&Vector3f::new(165.0, 165.0, 165.0))),
                Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::WHITE)))),
            )),
            Box::new(Transform::translation(Vector3f::new(-100.0, -195.0, 100.0)) * Transform::rotation_y(18.0 * PI / 180.0)),
        )))
        // Light
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                Arc::new(Rectangle::new(130.0, 105.0)),
                Arc::new(DiffuseLight::new(Arc::new(PlainColor::new(Spectrum::new(15.0, 15.0, 15.0))))),
            )),
            Box::new(Transform::translation(Vector3f::new(0.0, 276.0, 0.0)) * Transform::rotation_z(PI)),
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
