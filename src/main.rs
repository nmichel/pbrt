use pbrt::cameras::{Camera, PinHoleCamera, ThinLensCamera};
use pbrt::config::Config;
use pbrt::geom::bounds2::Bounds2;
use pbrt::geom::matrix4::Matrix4;
use pbrt::geom::transform::Transform;
use pbrt::geom::vector2::{Vector2, Vector2u, Vector2f};
use pbrt::geom::vector3::Vector3f;
use pbrt::integrators::integrator::Integrator;
use pbrt::integrators::path::PathIntegrator;
use pbrt::materials::{Dielectric, DiffuseLight, Lambertian, Material, Metal};
use pbrt::light::PointLight;
use pbrt::primitives::Primitive;
use pbrt::scene::Scene;
use pbrt::shapes::{csg, Plane, Rectangle, Sphere};
use pbrt::spectrum::Spectrum;
use pbrt::textures::*;
use rand::distributions::{IndependentSample, Range};
use std::env;
use std::f64;
use std::mem;
use std::process;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

enum Request {
    Compute { coords: Vector2u },
    Quit
}

struct Response { coords: Vector2u, spectrum: Spectrum }

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);        
    });

    println!("Redering with configuration settings: {:#?}", &config);

    // let scene = build_block_scene();
    // let scene = build_intersection_scene();
    let scene = build_union_scene();
    // let scene = build_my_balls();
    // let scene = build_cornel_box();

    let image_width = config.output_width as u32;
    let image_height = config.output_height as u32;
    let fov = config.fov_deg * f64::consts::PI / 180.0;
    let near = config.near;
    let far = config.far;
    let max_depth = config.max_depth;

    let resolution = Vector2u::new(image_width, image_height);
    // let cam_to_world = Matrix4::look_at(&Vector3f::new(6.0, 8.0, 15.0), &Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(0.0, 1.0, 0.0)); // CSG box
    // let cam_to_world = Matrix4::look_at(&Vector3f::new(1.0, 1.0, 1.0), &Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(0.0, 1.0, 0.0)); // CSG intersection
    let cam_to_world = Matrix4::look_at(&Vector3f::new(1.0, 2.0, 3.0), &Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(0.0, 1.0, 0.0)); // CSG union
    // let cam_to_world = Matrix4::look_at(&Vector3f::new(13.0, 2.0, 3.0), &Vector3f::new(0.0, 1.0, 0.0), &Vector3f::new(0.0, 1.0, 0.0)); // balls
    // let cam_to_world = Matrix4::look_at(&Vector3f::new(0.0, 0.0, 1178.0), &Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(0.0, 1.0, 0.0)); // Cornell
    let camera: Box<dyn Camera> = if config.lens_radius > 0.0 { Box::new(ThinLensCamera::new(&resolution, fov, near, far, config.lens_radius, config.focal_distance, cam_to_world)) } else { Box::new(PinHoleCamera::new(&resolution, fov, near, far, cam_to_world)) };
    let integrator = PathIntegrator::new(max_depth);

    let mut pixels:Vec<u8> = Vec::with_capacity((image_width * image_height * 4) as usize);
    for _ in 0..(image_width * image_height * 4) {
        pixels.push(0);
    }

    let patch = Bounds2::new(&Vector2::new(0, 0), &resolution);
    let pixel_count = image_width * image_height as u32;

    let config_ptr_transmuted = unsafe { mem::transmute::<&Config, &'static Config>(&config) };
    let integrator_ptr_transmuted = unsafe { mem::transmute::<&dyn Integrator, &'static dyn Integrator>(&integrator) };
    let scene_ptr_transmuted = unsafe { mem::transmute::<&Scene, &'static Scene>(&scene) };
    let camera_ptr_transmuted = unsafe { mem::transmute::<&dyn Camera, &'static dyn Camera>(&*camera) };

    let (upstream_tx, upstream_rx):(mpsc::Sender<Response>, mpsc::Receiver<Response>) = mpsc::channel();
    let mut handles: Vec<thread::JoinHandle<()>> = Vec::new();
    let mut senders: Vec<mpsc::Sender<Request>> = Vec::new();

    for i in 0..(config.threads) {
        let (tx, rx):(mpsc::Sender<Request>, mpsc::Receiver<Request>) = mpsc::channel();
        let upstream_tx = upstream_tx.clone();

        let config_ptr_transmuted = config_ptr_transmuted;
        let scene_ptr_transmuted = scene_ptr_transmuted;
        let integrator_ptr_transmuted = integrator_ptr_transmuted;
        let camera_ptr_transmuted = camera_ptr_transmuted;

        let handle = thread::spawn(move || {
            println!("Start thread {:?}", i);

            let rx = rx;
            let mut run = true;
            let mut sample = Sampler2::new();
        
            while run {
                match rx.recv().unwrap() {
                    Request::Quit => {
                        println!("[{:?}] QUIT !", i);
                        run = false;
                    }

                    Request::Compute { coords } => {
                        let spectrum = compute_pixel(&config_ptr_transmuted, integrator_ptr_transmuted, coords, camera_ptr_transmuted, scene_ptr_transmuted, &mut sample);
                        let response = Response { coords, spectrum };
                        upstream_tx.send(response).unwrap();
                    }
                }

            }
            println!("End thread {:?}", i);
        });
        handles.push(handle);
        senders.push(tx);
    }

    let mut pixel_iter = patch.to_iter();
    let mut pixel_computed = 0;
    let mut tid = 0;
    while pixel_computed < image_width * image_height {
        match pixel_iter.next() {
            None => {}
            Some(coords) => {
                senders[tid].send(Request::Compute{coords}).unwrap();
                tid = (tid + 1) % config.threads;
            }
        }

        match upstream_rx.try_recv() {
            Ok(Response{ coords, spectrum }) => {
                let mut res = spectrum;
                res.gamma_correct();
                let sample = res.to_rgb();
                let pixel_index = ((coords.y * image_width + coords.x) * 4) as usize;
                pixels[pixel_index] = sample[0];
                pixels[pixel_index+1] = sample[1];
                pixels[pixel_index+2] = sample[2];
                pixels[pixel_index+3] = sample[3];
                pixel_computed = pixel_computed + 1;

                print!("done [{:?}]\r", (pixel_computed as f64 / pixel_count as f64 * 100.0) as u32);
            }
            Err(_) => {}
        }
    }

    for i in 0..(config.threads) {
        println!("Sending QUIT order to [{:?}]", i);
        senders[i].send(Request::Quit).unwrap();
    }

    handles.drain(..).for_each(|handle| { handle.join().unwrap(); () });

    image_write(&config.output_filename, &resolution, &pixels);
}

fn compute_pixel(config: &Config, integrator: &Integrator, pixel_coords: Vector2<u32>, camera: &Camera, scene: &Scene, sampler: &mut Sampler2) -> Spectrum {
    let mut ns = config.samples_ppx;
    let mut res = Spectrum::new(0.0, 0.0, 0.0);
    let pixel_coords = Vector2f::from(pixel_coords);
    while ns > 0 {
        let pixel_coords = pixel_coords + sampler.sample();
        let ray = camera.get_ray(pixel_coords.x, pixel_coords.y);
        res += integrator.li(&ray, &scene, config.max_depth, config.near, config.far);
        ns -= 1;
    }
    res * (1.0/(config.samples_ppx as f64))
}

pub struct Sampler2 {
    rng: rand::ThreadRng,
    range: rand::distributions::Range<f64>
}

impl Sampler2 {
    pub fn new() -> Self {
        Sampler2 { range: Range::new(0., 1.), rng: rand::thread_rng()}
    }

    pub fn sample(&mut self) -> Vector2f {
        let x = self.range.ind_sample(&mut self.rng);
        let y = self.range.ind_sample(&mut self.rng);
        Vector2f { x, y }
    }
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

fn build_my_balls() -> Scene {
    let text_check_red: Arc<Texture> = Arc::new(CheckerBoard::new(Spectrum::new(0.65, 0.0, 0.0), Spectrum::new(0.65, 0.65, 0.65), 1000.0));
    let material_wall: Arc<Material> = Arc::new(Lambertian::new(Arc::clone(&text_check_red)));
    let material_lambertian: Arc<Material> = Arc::new(Lambertian::new(Arc::new(PlainColor::new(Spectrum::new(0.4, 0.2, 0.1)))));
    let material_dielectric: Arc<Material> = Arc::new(Dielectric::new(1.5, Arc::new(PlainColor::new(Spectrum::new(0.5, 0.6, 0.1)))));
    let material_metal_2: Arc<Material> = Arc::new(Metal::new(0.0, Arc::new(PlainColor::new(Spectrum::new(0.95, 0.6, 0.5)))));

    let mut scene = Scene::new();
    scene
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(4.0, 4.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, 4.0, 0.0))),
            Arc::new(DiffuseLight::new(Arc::new(PlainColor::new(Spectrum::new(4.0, 4.0, 4.0))))))))
        .add_object(Arc::new(Primitive::new(
            Box::new(Sphere::new(1000.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, -1000.0, 0.0))),
            Arc::clone(&material_wall))))
        .add_object(Arc::new(Primitive::new(
            Box::new(Sphere::new(1.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, 1.0, 0.0)) * Transform::rotation_x(0.3) * Transform::rotation_y(0.3)),
            Arc::clone(&material_dielectric))))
        .add_object(Arc::new(Primitive::new(
            Box::new(Sphere::new(1.0)),
            Box::new(Transform::translation(Vector3f::new(-4.0, 1.0, 0.0)) * Transform::rotation_x(0.3) * Transform::rotation_y(0.3)),
            Arc::clone(&material_lambertian))))
        .add_object(Arc::new(Primitive::new(
            Box::new(Sphere::new(1.0)),
            Box::new(Transform::translation(Vector3f::new(4.0, 1.0, 0.0)) * Transform::rotation_x(0.3) * Transform::rotation_y(0.3)),
            Arc::clone(&material_metal_2))))
        ;

    scene
}

fn build_cornel_box() -> Scene {
    let mut scene = Scene::new();
    scene
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(130.0, 105.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, 276.0, 0.0))),
            Arc::new(DiffuseLight::new(Arc::new(PlainColor::new(Spectrum::new(15.0, 15.0, 15.0))))))))
        // Floor
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(555.0, 555.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, -277.0, 0.0))),
            Arc::new(Lambertian::new(Arc::new(PlainColor::new(Spectrum::new(1.0, 1.0, 1.0))))))))
        // Ceiling
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(555.0, 555.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, 277.0, 0.0)) * Transform::rotation_x(-std::f64::consts::PI)),
            Arc::new(Lambertian::new(Arc::new(PlainColor::new(Spectrum::new(1.0, 1.0, 1.0))))))))
        // Right Wall
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(555.0, 555.0)),
            Box::new(Transform::translation(Vector3f::new(-277.0, 0.0, 0.0)) * Transform::rotation_z(-std::f64::consts::PI/2.0)),
            Arc::new(Lambertian::new(Arc::new(PlainColor::new(Spectrum::new(1.0, 0.0, 0.0))))))))
        // Left Wall
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(555.0, 555.0)),
            Box::new(Transform::translation(Vector3f::new(277.0, 0.0, 0.0)) * Transform::rotation_z(std::f64::consts::PI/2.0)),
            Arc::new(Lambertian::new(Arc::new(PlainColor::new(Spectrum::new(0.0, 1.0, 0.0))))))))
        // Back Wall
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(555.0, 555.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, 0.0, -277.0)) * Transform::rotation_x(std::f64::consts::PI/2.0)),
            Arc::new(Lambertian::new(Arc::new(PlainColor::new(Spectrum::new(1.0, 1.0, 1.0))))))))

        // Fuck Cornell ! Lets add some balls !
        .add_object(Arc::new(Primitive::new(
            Box::new(Sphere::new(90.0)),
            Box::new(Transform::translation(Vector3f::new(10.0, -100.0, 0.0)) * Transform::rotation_x(0.3) * Transform::rotation_y(0.3)),
            Arc::new(Dielectric::new(1.5, Arc::new(PlainColor::new(Spectrum::new(0.5, 0.6, 0.1))))))))
        .add_object(Arc::new(Primitive::new(
            Box::new(Sphere::new(60.0)),
            Box::new(Transform::translation(Vector3f::new(50.0, 90.0, 0.0)) * Transform::rotation_x(0.3) * Transform::rotation_y(0.3)),
            Arc::new(Lambertian::new(Arc::new(PlainColor::new(Spectrum::new(0.4, 0.2, 0.1))))))))
        .add_object(Arc::new(Primitive::new(
            Box::new(Sphere::new(40.0)),
            Box::new(Transform::translation(Vector3f::new(-100.0, 50.0, 100.0)) * Transform::rotation_x(0.3) * Transform::rotation_y(0.3)),
            Arc::new(Metal::new(0.0, Arc::new(PlainColor::new(Spectrum::new(0.95, 0.6, 0.5))))))))
        .add_object(Arc::new(Primitive::new(
            Box::new(Sphere::new(100.0)),
            Box::new(Transform::translation(Vector3f::new(40.0, -150.0, -130.0)) * Transform::rotation_x(0.3) * Transform::rotation_y(0.3)),
            Arc::new(Metal::new(0.0, Arc::new(PlainColor::new(Spectrum::new(0.95, 0.6, 0.5))))))))
        ;
    scene
}

fn build_union_scene() -> Scene {
    let mut scene = Scene::new();
    let elements = vec![
        Box::new(csg::Elem { shape: Box::new(Sphere::new(0.4)), transform: Box::new(Transform::translation(Vector3f::new(-0.5, 0.0, -0.5))) }),
        Box::new(csg::Elem { shape: Box::new(Sphere::new(0.4)), transform: Box::new(Transform::translation(Vector3f::new(0.0, 0.0, -0.5))) }),
        Box::new(csg::Elem { shape: Box::new(Sphere::new(0.4)), transform: Box::new(Transform::translation(Vector3f::new(0.5, 0.0, -0.5))) }),
        Box::new(csg::Elem { shape: Box::new(Sphere::new(0.4)), transform: Box::new(Transform::translation(Vector3f::new(-0.5, 0.0, 0.0))) }),
        Box::new(csg::Elem { shape: Box::new(Sphere::new(0.4)), transform: Box::new(Transform::translation(Vector3f::new(0.0, 0.0, 0.0))) }),
        Box::new(csg::Elem { shape: Box::new(Sphere::new(0.4)), transform: Box::new(Transform::translation(Vector3f::new(0.5, 0.0, 0.0))) }),
        Box::new(csg::Elem { shape: Box::new(Sphere::new(0.4)), transform: Box::new(Transform::translation(Vector3f::new(-0.5, 0.0, 0.5))) }),
        Box::new(csg::Elem { shape: Box::new(Sphere::new(0.4)), transform: Box::new(Transform::translation(Vector3f::new(0.0, 0.0, 0.5))) }),
        Box::new(csg::Elem { shape: Box::new(Sphere::new(0.4)), transform: Box::new(Transform::translation(Vector3f::new(0.5, 0.0, 0.5))) }),
        ];
    
    scene
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(4.0, 4.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, 4.0, 0.0))),
            Arc::new(DiffuseLight::new(Arc::new(PlainColor::new(Spectrum::new(4.0, 4.0, 4.0))))))))
        .add_object(Arc::new(Primitive::new(
            Box::new(csg::Union::new(elements)),
            Box::new(Transform::translation(Vector3f::new(0.0, 0.0, 0.0))),
            // Arc::new(Lambertian::new(Arc::new(PlainColor::new(Spectrum::new(0.4, 0.2, 0.1))))))))
            Arc::new(Dielectric::new(1.5, Arc::new(PlainColor::new(Spectrum::new(0.95, 0.95, 1.0))))))))
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(3.0, 3.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, -1.0, 0.0))),
            Arc::new(Lambertian::new(Arc::new(CheckerBoard::new(Spectrum::new(0.65, 0.0, 0.0), Spectrum::new(0.65, 0.65, 0.65), 2.0)))))))
        ;

    scene
}

fn build_intersection_scene() -> Scene {
    let mut scene = Scene::new();
    let elements = vec![
        Box::new(csg::Elem { shape: Box::new(Sphere::new(0.4)), transform: Box::new(Transform::translation(Vector3f::new(-0.2, 0.0, 0.0))) }),
        Box::new(csg::Elem { shape: Box::new(Sphere::new(0.4)), transform: Box::new(Transform::translation(Vector3f::new(0.2, 0.0, 0.0))) }),
        ];
    
    scene
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(4.0, 4.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, 4.0, 0.0))),
            Arc::new(DiffuseLight::new(Arc::new(PlainColor::new(Spectrum::new(4.0, 4.0, 4.0))))))))
        .add_object(Arc::new(Primitive::new(
            Box::new(csg::Intersection::new(elements)),
            Box::new(Transform::translation(Vector3f::new(0.0, 0.0, 0.0))),
            // Arc::new(Lambertian::new(Arc::new(PlainColor::new(Spectrum::new(0.4, 0.2, 0.1))))))))
            Arc::new(Dielectric::new(1.5, Arc::new(PlainColor::new(Spectrum::new(0.95, 0.95, 1.0))))))))
        // .add_object(Arc::new(Primitive::new(
        //     Box::new(Sphere::new(0.4)),
        //     Box::new(Transform::translation(Vector3f::new(-0.2, 0.0, 0.0))),
        //     // Arc::new(Lambertian::new(Arc::new(PlainColor::new(Spectrum::new(0.4, 0.2, 0.1))))))))
        //     Arc::new(Dielectric::new(1.5, Arc::new(PlainColor::new(Spectrum::new(0.95, 0.95, 1.0))))))))
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(3.0, 3.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, -1.0, 0.0))),
            Arc::new(Lambertian::new(Arc::new(CheckerBoard::new(Spectrum::new(0.65, 0.0, 0.0), Spectrum::new(0.65, 0.65, 0.65), 2.0)))))))
        ;

    scene
}



fn build_block_scene() -> Scene {
    let mut scene = Scene::new();
    let elements = vec![
        Box::new(csg::Elem { shape: Box::new(Plane::new()), transform: Box::new(Transform::translation(Vector3f::new(0.0, 2.0, 0.0))) }), // top
        Box::new(csg::Elem { shape: Box::new(Plane::new()), transform: Box::new(Transform::translation(Vector3f::new(0.0, -2.0, 0.0)) * Transform::rotation_x(-std::f64::consts::PI)) }), // bottom

        Box::new(csg::Elem { shape: Box::new(Plane::new()), transform: Box::new(Transform::translation(Vector3f::new(2.0, 0.0, 0.0)) * Transform::rotation_z(-std::f64::consts::PI/2.0)) }), // left
        Box::new(csg::Elem { shape: Box::new(Plane::new()), transform: Box::new(Transform::translation(Vector3f::new(-2.0, 0.0, 0.0)) * Transform::rotation_z(std::f64::consts::PI/2.0)) }), // right

        Box::new(csg::Elem { shape: Box::new(Plane::new()), transform: Box::new(Transform::translation(Vector3f::new(0.0, 0.0, -2.0)) * Transform::rotation_x(-std::f64::consts::PI/2.0)) }), // front
        Box::new(csg::Elem { shape: Box::new(Plane::new()), transform: Box::new(Transform::translation(Vector3f::new(0.0, 0.0, 2.0)) * Transform::rotation_x(std::f64::consts::PI/2.0)) }), // back
        ];
    
    scene
        .add_object(Arc::new(Primitive::new(
            Box::new(Rectangle::new(5.0, 5.0)),
            Box::new(Transform::translation(Vector3f::new(0.0, 10.0, 0.0))),
            Arc::new(DiffuseLight::new(Arc::new(PlainColor::new(Spectrum::new(4.0, 4.0, 4.0))))))))
        .add_object(Arc::new(Primitive::new(
            Box::new(csg::Intersection::new(elements)),
            Box::new(Transform::translation(Vector3f::new(0.0, 0.0, 0.0))),
            // Arc::new(Lambertian::new(Arc::new(PlainColor::new(Spectrum::new(0.6, 0.4, 0.1))))))))
            Arc::new(Dielectric::new(1.5, Arc::new(PlainColor::new(Spectrum::new(0.95, 0.95, 1.0))))))))
        .add_object(Arc::new(Primitive::new(
            Box::new(Plane::new()),
            Box::new(Transform::translation(Vector3f::new(0.0, -3.0, 0.0))),
            Arc::new(Lambertian::new(Arc::new(CheckerBoard::new(Spectrum::new(0.65, 0.0, 0.0), Spectrum::new(0.65, 0.65, 0.65), 0.5)))))))
        ;

    scene
}
