use pbrt::camera::{Camera, PinHoleCamera};
use pbrt::config::Config;
use pbrt::geom::bounds2::Bounds2;
use pbrt::geom::intersectable::{Intersectable, Intersection};
use pbrt::geom::ray::Ray;
use pbrt::geom::sphere::Sphere;
use pbrt::geom::vector2::{Vector2, Vector2u};
use pbrt::geom::vector3;
use pbrt::geom::vector3::Vector3f;
use pbrt::light::PointLight;
use pbrt::scene::Scene;
use pbrt::spectrum::Spectrum;
use std::env;
use std::f64;
use std::process;

pub trait Integrator {
    fn li(&self, ray: &Ray, scene: &Scene, depth: usize) -> Spectrum;
}

struct WhittedIntegrator {
    // Max recursion depth
    max_depth: usize
}

impl WhittedIntegrator {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }
}

impl Integrator for WhittedIntegrator {
    fn li(&self, ray: &Ray, scene: &Scene, depth: usize) -> Spectrum {
        match scene.intersect(&ray) {
            Some(Intersection { n, wo, .. }) => {
                let s = vector3::dot(&wo, &n);
                Spectrum::new(s, s, s)
            },
            None => {
                scene.background_radiance(&ray)
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);        
    });

    let mut scene = Scene::new();
    // scene
    //     .add_object(Box::new(Sphere::new(Vector3f::new( 0.0,  2.0,  5.0), 1.)))
    //     .add_object(Box::new(Sphere::new(Vector3f::new( 0.0, -2.0,  7.0), 1.)))
    //     .add_object(Box::new(Sphere::new(Vector3f::new( 2.0,  2.0,  7.0), 1.)))
    //     .add_object(Box::new(Sphere::new(Vector3f::new(-2.0, -2.0,  10.0), 1.)))
    //     .add_object(Box::new(Sphere::new(Vector3f::new( 0.0,  0.0, -10.0), 1.))) // behind the camera
    //     ;

    scene
        .add_light(Box::new(PointLight::new(Vector3f::new( 0.0,  5.0,  5.0))))
        .add_object(Box::new(Sphere::new(Vector3f::new( 0.0,  0.0,  5.0), 1.)))
        .add_object(Box::new(Sphere::new(Vector3f::new( 0.0,  0.0, 10.0), 1.)))
        .add_object(Box::new(Sphere::new(Vector3f::new( 3.0, -2.0, 10.0), 1.)))
        .add_object(Box::new(Sphere::new(Vector3f::new( 3.0,  2.0,  5.0), 1.)))
        ;

    let image_width = config.output_width as u32;
    let image_height = config.output_height as u32;
    let fov = config.fov_deg * f64::consts::PI / 180.0;
    let near = config.near;
    let far = config.far;
    let max_depth = config.max_depth;

    let resolution = Vector2u::new(image_width, image_height);
    let camera = PinHoleCamera::new(&resolution, fov, near, far);
    let integrator = WhittedIntegrator::new(max_depth);
    let mut pixels:Vec<u8> = Vec::new();
    let patch = Bounds2::new(&Vector2::new(0, 0), &resolution);
    for pixel_coords in patch.to_iter() {
        let ray = camera.get_ray(&pixel_coords);
        let spectrum = integrator.li(&ray, &scene, 3);
        let mut sample = spectrum.to_rgb();
        pixels.append(&mut sample);
    }

    image_write(&config.output_filename, &resolution, &pixels);
}

fn image_write(filename: &str, resolution: &Vector2u, data: &Vec<u8>) {
    use std::path::Path;
    use std::fs::File;
    use std::io::BufWriter;
    use png::HasParameters;

    let path = Path::new(filename);
    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, resolution.x, resolution.y);
    encoder
        .set(png::ColorType::RGBA)
        .set(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();

    writer.write_image_data(&data[..]).unwrap();
}
