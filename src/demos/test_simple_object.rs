use std::f64;
use std::sync::Arc;

use crate::cameras::{Camera, PinHoleCamera};
use crate::config::Config;
use crate::geom::matrix4::Matrix4;
use crate::geom::vector2::Vector2u;
use crate::geom::vector3::Vector3f;
use crate::materials::Lambertian;
use crate::objects::Simple;
use crate::scene::Scene;
use crate::shapes::Sphere;
use crate::spectrum::Spectrum;
use crate::textures::*;

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

    scene.add_object(Arc::new(Simple::new(
        Arc::new(Sphere::new(1.0)),
        Arc::new(Lambertian::new(Arc::new(CheckerBoard::new(
            Spectrum::new(0.2, 0.3, 0.1),
            Spectrum::new(0.9, 0.9, 0.9),
            10.0,
        )))),
    )));

    (scene, Box::new(camera))
}
