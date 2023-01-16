#![cfg(target_os = "ignore")]

use num_traits::ToPrimitive;

use crate::cameras::{Camera, ThinLensCamera};
use crate::colors;
use crate::config::Config;
use crate::geom::matrix4::Matrix4;
use crate::geom::transform::Transform;
use crate::geom::vector2::Vector2u;
use crate::geom::vector3::Vector3f;
use crate::materials::{Dielectric, Lambertian, Material, Metal, RefractionIndices};
use crate::primitives::Primitive;
use crate::scene::Scene;
use crate::shapes::Sphere;
use crate::spectrum::Spectrum;
use crate::textures::*;
use crate::utils::random_double;
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
    // let fov = config.fov_deg * f64::consts::PI / 180.0;
    let fov = 20.0 * f64::consts::PI / 180.0;
    let near = config.near;
    let far = config.far;

    let resolution = Vector2u::new(image_width, image_height);
    let lookfrom: Vector3f = Vector3f::new(13.0, 2.0, 3.0);
    let lookat: Vector3f = Vector3f::new(0.0, 0.0, 0.0);
    let vup: Vector3f = Vector3f::new(0.0, 1.0, 0.0);
    let cam_to_world = Matrix4::look_at(&lookfrom, &lookat, &vup);

    let camera = ThinLensCamera::new(&resolution, fov, near, far, 0.1, 10.0, cam_to_world);

    // Scene
    //
    let mut scene = Scene::new();
    scene
        .add_object(Arc::new(Primitive::new(
            Box::new(Sphere::new(1000.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, -1000.0, 0.0))),
            Arc::new(Lambertian::new(Arc::new(CheckerBoard::new(
                Spectrum::new(0.2, 0.3, 0.1),
                Spectrum::new(0.9, 0.9, 0.9),
                2000.0,
            )))),
        )))
        .add_object(Arc::new(Primitive::new(
            Box::new(Sphere::new(1.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, 1.0, 0.0))),
            Arc::new(Dielectric::new(RefractionIndices::WATER, Arc::new(PlainColor::new(colors::WHITE)))),
        )))
        .add_object(Arc::new(Primitive::new(
            Box::new(Sphere::new(1.0)),
            Box::new(Transform::translation(Vector3f::new(-4.0, 1.0, 0.0))),
            Arc::new(Lambertian::new(Arc::new(PlainColor::new(Spectrum::new(0.4, 0.2, 0.1))))),
        )))
        .add_object(Arc::new(Primitive::new(
            Box::new(Sphere::new(1.0)),
            Box::new(Transform::translation(Vector3f::new(4.0, 1.0, 0.0))),
            Arc::new(Metal::new(0.0, Arc::new(PlainColor::new(Spectrum::new(0.7, 0.6, 0.5))))),
        )));

    for a in -11..10 {
        for b in -11..10 {
            let choose_mat = random_double();
            let material: Arc<dyn Material>;

            if choose_mat < 0.7 {
                let color = Spectrum::new(random_double(), random_double(), random_double());
                material = Arc::new(Lambertian::new(Arc::new(PlainColor::new(color))));
            }
            else if choose_mat < 0.85 {
                let color = Spectrum::new(random_double(), random_double(), random_double());
                material = Arc::new(Metal::new(0.0, Arc::new(PlainColor::new(color))));
            }
            else {
                material = Arc::new(Dielectric::new(RefractionIndices::WATER, Arc::new(PlainColor::new(colors::WHITE))));
            }

            let center = Vector3f::new(
                a.to_f64().unwrap() + 0.9 * random_double(),
                0.2,
                b.to_f64().unwrap() + 0.9 * random_double(),
            );
            scene.add_object(Arc::new(Primitive::new(
                Box::new(Sphere::new(0.2)),
                Box::new(Transform::translation(center)),
                material,
            )));
        }
    }

    (scene, Box::new(camera))
}
