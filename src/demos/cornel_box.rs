#![cfg(target_os = "ignore")]

use crate::cameras::{Camera, PinHoleCamera};
use crate::colors;
use crate::config::Config;
use crate::geom::matrix4::Matrix4;
use crate::geom::transform::{Transform, Transformable};
use crate::geom::vector2::Vector2u;
use crate::geom::vector3::Vector3f;
use crate::materials::{Dielectric, DiffuseLight, Lambertian, Material, Metal, RefractionIndices};
use crate::primitives::Primitive;
use crate::scene::Scene;
use crate::shapes::{AABox, Rectangle, Sphere};
use crate::spectrum::Spectrum;
use crate::textures::PlainColor;
use crate::utils::random_double;
use std::f64;
use std::f64::consts::{FRAC_PI_2, PI};
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
    // let fov = config.fov_deg * f64::consts::PI / 180.0;
    let fov = 60.0 * f64::consts::PI / 180.0;
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
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(555.0, 555.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, 277.0, 0.0)) * Transform::rotation_x(-PI)),
            Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::WHITE)))),
        )))
        // Floor
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(555.0, 555.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, -277.0, 0.0))),
            Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::WHITE)))),
        )))
        // Right
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(555.0, 555.0)),
            Box::new(Transform::translation(Vector3f::new(-277.0, 0.0, 0.0)) * Transform::rotation_z(-FRAC_PI_2)),
            Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::RED)))),
        )))
        // Left
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(555.0, 555.0)),
            Box::new(Transform::translation(Vector3f::new(277.0, 0.0, 0.0)) * Transform::rotation_z(FRAC_PI_2)),
            Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::GREEN)))),
        )))
        // Back
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(555.0, 555.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, 0.0, -277.0)) * Transform::rotation_x(FRAC_PI_2)),
            Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::WHITE)))),
        )))
        .add_object(Arc::new(Primitive::new(
            Box::new(AABox::new(&Vector3f::new(165.0, 330.0, 165.0))),
            Box::new(Transform::translation(Vector3f::new(80.0, -150.0, -100.0)) * Transform::rotation_y(-15.0 * PI / 180.0)),
            Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::WHITE)))),
        )))
        .add_object(Arc::new(Primitive::new(
            Box::new(AABox::new(&Vector3f::new(165.0, 165.0, 165.0))),
            Box::new(Transform::translation(Vector3f::new(-100.0, -195.0, 100.0)) * Transform::rotation_y(18.0 * PI / 180.0)),
            Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::WHITE)))),
        )))
        // Light
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(130.0, 105.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, 276.0, 0.0)) * Transform::rotation_z(PI)),
            Arc::new(DiffuseLight::new(Arc::new(PlainColor::new(Spectrum::new(15.0, 15.0, 15.0))))),
        )));

    (scene, Box::new(camera))
}
