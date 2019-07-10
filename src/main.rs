use pbrt::config::Config;
use pbrt::geom::intersectable::Intersectable;
use pbrt::geom::ray::Ray;
use pbrt::geom::sphere::Sphere;
use pbrt::geom::vector3::Vector3;
use pbrt::scene::Scene;
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);        
    });

    let sphere = Sphere::new(Vector3::new(0., 0., 0.), 1.0);
    let ray = Ray {};
    let _collisions = sphere.intersect(&ray);

    let mut scene = Scene::new();
    scene
        .add(Box::new(Sphere::new(Vector3::new(0., 0., 0.), 1.0)))
        .add(Box::new(Sphere::new(Vector3::new(0., 10., 0.), 1.0)));

    scene.intersect(&ray).iter().for_each(|item| {
        println!("Intersection {:?}", item);
    });

    let mut pixels:Vec<u8> = Vec::new();
    let width: u64 = 1920;
    let height: u64 = 1024;
    for row in 0..height {
        let r = (((row+1) as f64 / height as f64) * 255.0) as u8;
        for col in 0..width {
            let g = (((col+1) as f64 / width as f64) * 255.0) as u8;
            let mut sample = vec![r, g, 0, 255];
            pixels.append(&mut sample);
        }
    }

    image_write(&config.output_filename, &pixels);
}

fn image_write(filename: &str, data: &Vec<u8>) {
    use std::path::Path;
    use std::fs::File;
    use std::io::BufWriter;
    use png::HasParameters;

    let path = Path::new(filename);
    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, 1920, 1024);
    encoder
        .set(png::ColorType::RGBA)
        .set(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();

    writer.write_image_data(&data[..]).unwrap();
}
