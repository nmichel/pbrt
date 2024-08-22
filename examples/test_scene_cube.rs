use num_traits::ToPrimitive;
use pbrt::cameras::{Camera, ThinLensCamera};
use pbrt::config::Config;
use pbrt::geom::matrix4::Matrix4;
use pbrt::geom::transform::Transform;
use pbrt::geom::vector2::Vector2u;
use pbrt::geom::vector3::Vector3f;
use pbrt::integrators::{self, Integrator, NormalIntegrator, PathIntegrator};
use pbrt::materials::*;
use pbrt::objects::{Simple, Transformed};
use pbrt::scene::Scene;
use pbrt::shapes::{AABox, Sphere};
use pbrt::spectrum::Spectrum;
use pbrt::textures::PlainColor;
use pbrt::utils::random_double;
use pbrt::{colors, renderers};
use std::f64::consts::FRAC_PI_3;
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
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                Arc::new(Sphere::new(1000.0)),
                Arc::new(Lambertian::new(Arc::new(PlainColor::new(Spectrum::new(0.5, 0.5, 0.5))))),
            )),
            Box::new(Transform::translation(Vector3f::new(0.0, -1000.0, 0.0))),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                Arc::new(AABox::new(&Vector3f::new(0.8, 0.8, 0.8))),
                Arc::new(Dielectric::new(RefractionIndices::WATER, Arc::new(PlainColor::new(colors::WHITE)))),
            )),
            Box::new(Transform::translation(Vector3f::new(0.0, 1.0, 0.0)) * Transform::rotation_x(FRAC_PI_3)),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                Arc::new(AABox::new(&Vector3f::new(0.8, 0.8, 0.8))),
                Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::DARK_RED)))),
            )),
            Box::new(Transform::translation(Vector3f::new(-4.0, 1.0, 0.0)) * Transform::rotation_y(FRAC_PI_3)),
        )))
        .add_object(Arc::new(Transformed::new(
            Arc::new(Simple::new(
                Arc::new(AABox::new(&Vector3f::new(0.8, 0.8, 0.8))),
                Arc::new(Metal::new(0.0, Arc::new(PlainColor::new(colors::YELLOW_GREEN)))),
            )),
            Box::new(Transform::translation(Vector3f::new(4.0, 1.0, 0.0)) * Transform::rotation_z(FRAC_PI_3)),
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
            scene.add_object(Arc::new(Transformed::new(
                Arc::new(Simple::new(Arc::new(Sphere::new(0.2)), material)),
                Box::new(Transform::translation(center)),
            )));
        }
    }

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
