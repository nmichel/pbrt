use crate::cameras::{Camera, PinHoleCamera};
use crate::colors;
use crate::config::Config;
use crate::geom::matrix4::Matrix4;
use crate::geom::transform::Transform;
use crate::geom::vector2::Vector2u;
use crate::geom::vector3::Vector3f;
use crate::materials::{Dielectric, Lambertian, Material, Metal, RefractionIndices};
use crate::primitives::Primitive;
use crate::scene::Scene;
use crate::shapes::{AABox, Sphere};
use crate::spectrum::Spectrum;
use crate::textures::*;
use std::f64;
use std::sync::Arc;

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
        )));

    (scene, Box::new(camera))
}
