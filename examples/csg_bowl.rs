use pbrt::cameras::{Camera, PinHoleCamera};
use pbrt::config::Config;
use pbrt::geom::matrix4::Matrix4;
use pbrt::geom::transform::Transform;
use pbrt::geom::vector2::Vector2u;
use pbrt::geom::vector3::Vector3f;
use pbrt::integrators::{self, Integrator, NaiveIntegrator, NormalIntegrator, PathIntegrator};
use pbrt::lights::BackgroundInfiniteLight;
use pbrt::materials::*;
use pbrt::objects::{Simple, Transformed};
use pbrt::scene::Scene;
use pbrt::shapes::{csg, AABox, Rectangle, Sphere};
use pbrt::spectrum::Spectrum;
use pbrt::textures::{CheckerBoard, PlainColor};
use pbrt::{colors, renderers};
use std::sync::Arc;
use std::{env, f64, process};

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
        &Vector3f::new(1.5, 1.1, 1.8),
        &Vector3f::new(0.0, -1.0, 0.0),
        &Vector3f::new(0.0, 1.0, 0.0),
    );

    let camera = PinHoleCamera::new(&resolution, fov, near, far, cam_to_world);

    // Scene
    //
    let mut scene = Scene::new();

    let bowl = csg::Intersection::new(vec![
        Box::new(csg::Elem {
            shape: Arc::new(Sphere::new(1.0)),
            transform: Box::new(Transform::translation(Vector3f::new(0.5, 0.0, 0.0))),
        }),
        Box::new(csg::Elem {
            shape: Arc::new(AABox::new(&Vector3f::new(1.0, 1.0, 1.0))),
            transform: Box::new(Transform::translation(Vector3f::new(0.0, 0.0, 0.0))),
        }),
    ]);

    scene.add_object(Arc::new(Transformed::new(
        Arc::new(Simple::new(
            Arc::new(bowl),
            Arc::new(Dielectric::new(RefractionIndices::GLASS, Arc::new(PlainColor::new(colors::WHITE)))),
        )),
        Box::new(Transform::translation(Vector3f::new(0.0, 0.0, 0.0))),
    )));

    scene.add_object(Arc::new(Transformed::new(
        Arc::new(Simple::new(
            Arc::new(Rectangle::new(6.0, 6.0)),
            Arc::new(Lambertian::new(Arc::new(CheckerBoard::new(
                Spectrum::new(0.65, 0.0, 0.0),
                Spectrum::new(0.65, 0.65, 0.65),
                2.0,
            )))),
        )),
        Box::new(Transform::translation(Vector3f::new(0.0, -0.6, 0.0))),
    )));

    // Nothing in this scene emits, and until this line no `Light` is registered either — which
    // under the default `PATH` integrator is enough to make the image uniformly black. The chain
    // is worth spelling out, because every link fails silently:
    //
    //   [1] `sample_light` returns `None` as soon as `Scene::lights` is empty, so next-event
    //       estimation contributes nothing.
    //   [2] `background_radiance` sums `le` over the *infinite* lights. Over an empty set that
    //       sum is BLACK, so a ray that escapes the scene carries nothing back.
    //   [3] A material's own emission is only accumulated while `is_last_bounce_specular`, so
    //       even an emissive surface would go dark after the first diffuse bounce.
    //
    // A `BackgroundInfiniteLight` answers all three. It is the sky of "Ray Tracing in One
    // Weekend", interpolating on the vertical component of the direction — `f` at the nadir, `t`
    // at the zenith. The gradient below is the one `NaiveIntegrator` hard-codes, which is what
    // makes `--integrator path` and `--integrator naive` comparable on the same scene.
    //
    // It feeds [1] as well as [2]: `sample_li` genuinely samples, drawing `wi` from a `SpherePdf`.
    // Departure from the ideal, worth naming — that pdf is uniform over the *whole* sphere,
    // including the half below the horizon no surface can see, so about half the samples land on
    // an invisible hemisphere. The estimator stays unbiased; only the variance pays.
    scene.add_light(Arc::new(BackgroundInfiniteLight::new(colors::WHITE, Spectrum::new(0.5, 0.7, 1.0))));

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
        integrators::Type::NAIVE => Box::new(NaiveIntegrator::new(config.max_depth)),
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
