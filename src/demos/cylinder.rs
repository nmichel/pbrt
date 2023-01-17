use crate::cameras::{Camera, PinHoleCamera};
use crate::colors;
use crate::config::Config;
use crate::geom::matrix4::Matrix4;
use crate::geom::transform::Transform;
use crate::geom::vector2::Vector2u;
use crate::geom::vector3::Vector3f;
use crate::materials::{Dielectric, Lambertian, Metal, RefractionIndices};
use crate::objects::{Simple, Transformed};
use crate::scene::Scene;
use crate::shapes::{Cylinder, Sphere};
use crate::spectrum::Spectrum;
use crate::textures::*;
use core::f64::consts::{FRAC_PI_3, FRAC_PI_4, FRAC_PI_6};
use std::f64;
use std::sync::Arc;

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
        &Vector3f::new(1.0, 1.8, 2.5),
        &Vector3f::new(0.0, -1.3, 0.0),
        &Vector3f::new(0.0, 1.0, 0.0),
    );

    let camera = PinHoleCamera::new(&resolution, fov, near, far, cam_to_world);

    // Scene
    //
    let mut scene = Scene::new();

    let offset = 1.5;

    let cylinder_shape = Arc::new(Cylinder::new(0.45, 1.0));

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
            Arc::new(Simple::new(
                cylinder_shape.clone(),
                Arc::new(Dielectric::new(RefractionIndices::GLASS, Arc::new(PlainColor::new(colors::WHITE)))),
            )),
            Box::new(Transform::translation(Vector3f::new(-offset, 0.0, -offset)) * Transform::rotation_z(-FRAC_PI_3)),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                cylinder_shape.clone(),
                Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::RED)))),
            )),
            Box::new(Transform::translation(Vector3f::new(0.0, 0.0, -offset)) * Transform::rotation_x(FRAC_PI_4)),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                cylinder_shape.clone(),
                Arc::new(Metal::new(0.0, Arc::new(PlainColor::new(colors::PEACH_PUFF)))),
            )),
            Box::new(
                Transform::translation(Vector3f::new(offset, 0.0, -offset)) * Transform::rotation_x(FRAC_PI_4) * Transform::rotation_z(FRAC_PI_3),
            ),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                cylinder_shape.clone(),
                Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::ORANGE)))),
            )),
            Box::new(Transform::translation(Vector3f::new(-offset, 0.0, 0.0))),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                cylinder_shape.clone(),
                Arc::new(Dielectric::new(RefractionIndices::GLASS, Arc::new(PlainColor::new(colors::WHITE)))),
            )),
            Box::new(Transform::translation(Vector3f::new(0.0, 0.0, 0.0)) * Transform::rotation_z(FRAC_PI_6)),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                cylinder_shape.clone(),
                Arc::new(Metal::new(0.0, Arc::new(PlainColor::new(colors::PEACH_PUFF)))),
            )),
            Box::new(Transform::translation(Vector3f::new(offset, 0.0, 0.0))),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                cylinder_shape.clone(),
                Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::BLUE)))),
            )),
            Box::new(Transform::translation(Vector3f::new(-offset, 0.0, offset))),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                cylinder_shape.clone(),
                Arc::new(Metal::new(0.0, Arc::new(PlainColor::new(colors::PEACH_PUFF)))),
            )),
            Box::new(Transform::translation(Vector3f::new(0.0, 0.0, offset)) * Transform::rotation_x(-FRAC_PI_4) * Transform::rotation_z(-FRAC_PI_3)),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                cylinder_shape.clone(),
                Arc::new(Dielectric::new(RefractionIndices::GLASS, Arc::new(PlainColor::new(colors::WHITE)))),
            )),
            Box::new(Transform::translation(Vector3f::new(offset, 0.0, offset))),
        )));

    (scene, Box::new(camera))
}
