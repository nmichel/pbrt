use crate::cameras::{Camera, PinHoleCamera};
use crate::colors;
use crate::config::Config;
use crate::geom::matrix4::Matrix4;
use crate::geom::transform::Transform;
use crate::geom::vector2::Vector2u;
use crate::geom::vector3::Vector3f;
use crate::materials::Lambertian;
use crate::objects::{Simple, Transformed};
use crate::scene::Scene;
use crate::shapes::{csg, Rectangle, Sphere};
use crate::spectrum::Spectrum;
use crate::textures::*;
use std::f64;
use std::sync::Arc;

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
