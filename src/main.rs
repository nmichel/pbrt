use pbrt::cameras::PinHoleCamera;
use pbrt::config::Config;
use pbrt::geom::matrix4::Matrix4;
use pbrt::geom::transform::Transform;
use pbrt::geom::vector2::Vector2u;
use pbrt::geom::vector3::Vector3f;
use pbrt::integrators::{NormalIntegrator, PathIntegrator};
use pbrt::materials::{Dielectric, DiffuseLight, Lambertian, Material, Metal, RefractionIndices};
use pbrt::primitives::Primitive;
use pbrt::renderers;
use pbrt::scene::Scene;
use pbrt::shapes::{AABox, Rectangle, Sphere};
use pbrt::spectrum::Spectrum;
use pbrt::textures::*;
use std::env;
use std::f64;
use std::process;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    println!("Redering with configuration settings: {:#?}", &config);

    let scene = build_scene();

    let image_width = config.output_width as u32;
    let image_height = config.output_height as u32;
    let fov = config.fov_deg * f64::consts::PI / 180.0;
    let near = config.near;
    let far = config.far;
    let max_depth = config.max_depth;

    let resolution = Vector2u::new(image_width, image_height);
    let cam_to_world = Matrix4::look_at(
        &Vector3f::new(0.7, 0.6, 0.5),
        &Vector3f::new(0.0, 0.0, -1.0),
        &Vector3f::new(0.0, 1.0, 0.0),
    );
    let camera = PinHoleCamera::new(&resolution, fov, near, far, cam_to_world);
    let integrator = PathIntegrator::new(max_depth);
    // let integrator = NormalIntegrator::new();

    renderers::mt::render(&config, &scene, &camera, &integrator);
}

fn build_scene() -> Scene {
    let text_check_red: Arc<dyn Texture> = Arc::new(CheckerBoard::new(
        Spectrum::new(0.65, 0.0, 0.0),
        Spectrum::new(0.65, 0.65, 0.65),
        1000.0,
    ));
    let material_wall: Arc<dyn Material> = Arc::new(Lambertian::new(Arc::clone(&text_check_red)));
    let material_lambertian: Arc<dyn Material> = Arc::new(Lambertian::new(Arc::new(
        PlainColor::new(Spectrum::ORANGE),
    )));
    let material_lambertian_white: Arc<dyn Material> = Arc::new(Lambertian::new(Arc::new(
        PlainColor::new(Spectrum::WHITE),
    )));
    let material_dielectric: Arc<dyn Material> = Arc::new(Dielectric::new(
        RefractionIndices::WATER,
        Arc::new(PlainColor::new(Spectrum::WHITE)),
    ));
    let material_metal_white: Arc<dyn Material> = Arc::new(Metal::new(
        0.0,
        Arc::new(PlainColor::new(Spectrum::WHITE)),
    ));

    let mut scene = Scene::new();
    scene
        // .add_object(Arc::new(Primitive::new(
        //     Box::new(Rectangle::new(4.0, 4.0)),
        //     Box::new(Transform::translation(Vector3f::new(0.0, 4.0, 0.0))),
        //     Arc::new(DiffuseLight::new(Arc::new(PlainColor::new(Spectrum::new(
        //         2.0, 2.0, 2.0,
        //     ))))),
        // )))
        .add_object(Arc::new(Primitive::new(
            Box::new(Sphere::new(100.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, -100.5, -1.0))),
            Arc::clone(&material_wall),
        )))
        .add_object(Arc::new(Primitive::new(
            Box::new(Sphere::new(0.5)),
            Box::new(Transform::translation(Vector3f::new(-1.0, 0.0, -1.0))),
            Arc::clone(&material_lambertian),
        )))
        .add_object(Arc::new(Primitive::new(
            Box::new(AABox::new(&Vector3f::new(0.8, 0.8, 0.8))),
            Box::new(Transform::translation(Vector3f::new(0.0, 0.0, -0.6))),
            Arc::clone(&material_dielectric),
        )))
        // .add_object(Arc::new(Primitive::new(
        //     Box::new(Sphere::new(0.5)),
        //     Box::new(Transform::translation(Vector3f::new(0.0, 0.0, -1.0))),
        //     Arc::clone(&material_lambertian),
        // )))
        .add_object(Arc::new(Primitive::new(
            Box::new(Sphere::new(0.5)),
            Box::new(Transform::translation(Vector3f::new(1.0, 0.0, -1.0))),
            Arc::clone(&material_metal_white),
        )))
        ;

    scene
}
