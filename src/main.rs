use pbrt::camera::{Camera, PinHoleCamera};
use pbrt::config::Config;
use pbrt::geom::bounds2::Bounds2;
use pbrt::geom::transform::Transform;
use pbrt::geom::vector2::{Vector2, Vector2u};
use pbrt::geom::vector3::Vector3f;
use pbrt::integrators::integrator::Integrator;
use pbrt::integrators::whitted::WhittedIntegrator;
use pbrt::materials::material::Material;
use pbrt::light::PointLight;
use pbrt::primitives::Primitive;
use pbrt::scene::Scene;
use pbrt::shapes::sphere::Sphere;
use pbrt::shapes::plane::Plane;
use pbrt::spectrum::Spectrum;
use pbrt::textures::*;
use rand::distributions::{IndependentSample, Range};    
use std::env;
use std::f64;
use std::process;
use std::rc::Rc;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);        
    });

    let text_plain_green: Rc<Texture> = Rc::new(PlainColor::new(Spectrum::new(1.022,0.782,0.344)));
    let text_check_red: Rc<Texture> = Rc::new(CheckerBoard::new(Spectrum::new(0.65, 0.0, 0.0), Spectrum::new(0.65, 0.65, 0.65), 4.0));
    let text_check_green: Rc<Texture> = Rc::new(CheckerBoard::new(Spectrum::new(0.0, 1.0, 0.0), Spectrum::new(0.1, 0.5, 0.0), 0.5));
    let shiny_ball: Rc<Material> = Rc::new(Material::new(Rc::clone(&text_check_red)));
    let shiny_ball_gold: Rc<Material> = Rc::new(Material::new(Rc::clone(&text_plain_green)));
    let material_wall: Rc<Material> = Rc::new(Material::new(Rc::clone(&text_check_green)));

    let mut scene = Scene::new();
    scene
        .add_light(Box::new(PointLight::new(
            Box::new(Transform::translation(Vector3f::new(-2.0, 2.0, 0.0))),
            Spectrum::new(1.0, 1.0, 1.0))))
        // .add_light(Box::new(PointLight::new(
        //     Box::new(Transform::translation(Vector3f::new(-2.0, 0.0, 1.0))),
        //     Spectrum::new(0.5, 0.5, 0.5))))
        .add_object(Box::new(Primitive::new(
            Box::new(Plane::new()),
            Box::new(Transform::translation(Vector3f::new(0.0, -1.0, 0.0))),
            Rc::clone(&material_wall))))
        .add_object(Box::new(Primitive::new(
            Box::new(Sphere::new(0.6)),
            Box::new(Transform::translation(Vector3f::new(0.0, 0.0, 2.0)) * Transform::rotation_x(0.3) * Transform::rotation_y(0.3)),
            Rc::clone(&shiny_ball_gold))))
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
    let between = Range::new(0., 1.);
    let mut rng = rand::thread_rng();
    for pixel_coords in patch.to_iter() {
        const SAMPLES: u32 = 10;
        let mut ns: u32 = SAMPLES;
        let mut res = Spectrum::new(0.0, 0.0, 0.0);
        while ns > 0 {
            let dx = between.ind_sample(&mut rng);
            let dy = between.ind_sample(&mut rng);
            let pixel_x = pixel_coords.x as f64 + dx;
            let pixel_y = pixel_coords.y as f64 + dy;
            let ray = camera.get_ray(pixel_x, pixel_y);
            res += integrator.li(&ray, &scene, 3);

            ns -= 1;
        }
        // res.gamma_correct();
        let mut sample = (res * (1.0/(SAMPLES as f64))).to_rgb();
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
