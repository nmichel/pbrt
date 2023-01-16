use std::f64;
use std::sync::Arc;

use crate::cameras::{Camera, PinHoleCamera};
use crate::colors;
use crate::config::Config;
use crate::geom::matrix4::Matrix4;
use crate::geom::transform::Transform;
use crate::geom::vector2::Vector2u;
use crate::geom::vector3::Vector3f;
use crate::materials::Lambertian;
use crate::objects::{Compound, Simple, Transformed};
use crate::scene::Scene;
use crate::shapes::{Cylinder, Sphere};
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
        )));
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
