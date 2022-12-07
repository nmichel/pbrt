use crate::cameras::Camera;
use crate::config::Config;
use crate::geom::bounds2::Bounds2;
use crate::geom::vector2::{Vector2, Vector2f, Vector2u};
use crate::integrators::Integrator;
use crate::scene::Scene;
use crate::spectrum::Spectrum;
use rand::distributions::{IndependentSample, Range};

pub fn render(config: &Config, scene: &Scene, camera: &dyn Camera, integrator: &dyn Integrator) {
    let image_width = config.output_width as u32;
    let image_height = config.output_height as u32;

    let mut pixels: Vec<u8> = Vec::new();
    pixels.resize((image_width * image_height * 4) as usize, 0);

    let resolution = Vector2u::new(image_width, image_height);
    let patch = Bounds2::new(&Vector2::new(0, 0), &resolution);
    let pixel_count = image_width * image_height as u32;

    let mut pixel_iter = patch.to_iter();
    let mut pixel_computed = 0;
    let mut sample = Sampler2::new();
    while pixel_computed < image_width * image_height {
        match pixel_iter.next() {
            None => {}
            Some(coords) => {
                // println!("\n\n* Pixel {:?}", &coords);
                let mut spectrum =
                    compute_pixel(config, integrator, coords, camera, scene, &mut sample);

                spectrum.gamma_correct();
                let sample = spectrum.to_rgb();
                let pixel_index = ((coords.y * image_width + coords.x) * 4) as usize;
                pixels[pixel_index] = sample[0];
                pixels[pixel_index + 1] = sample[1];
                pixels[pixel_index + 2] = sample[2];
                pixels[pixel_index + 3] = sample[3];
                pixel_computed = pixel_computed + 1;

                print!(
                    "done [{:?}]\r",
                    (pixel_computed as f64 / pixel_count as f64 * 100.0) as u32
                );
            }
        }
    }

    image_write(&config.output_filename, &resolution, &pixels);
}

fn compute_pixel(
    config: &Config,
    integrator: &dyn Integrator,
    pixel_coords: Vector2<u32>,
    camera: &dyn Camera,
    scene: &Scene,
    sampler: &mut Sampler2,
) -> Spectrum {
    let mut ns = config.samples_ppx;
    let mut res = Spectrum::new(0.0, 0.0, 0.0);
    let pixel_coords = Vector2f::from(pixel_coords);
    while ns > 0 {
        let pixel_coords = pixel_coords + sampler.sample();
        let ray = camera.get_ray(pixel_coords.x, pixel_coords.y);
        res += integrator.li(&ray, &scene, config.max_depth, config.near, config.far);
        ns -= 1;
    }
    res * (1.0 / (config.samples_ppx as f64))
}

pub struct Sampler2 {
    rng: rand::ThreadRng,
    range: rand::distributions::Range<f64>,
}

impl Sampler2 {
    pub fn new() -> Self {
        Sampler2 {
            range: Range::new(0., 1.),
            rng: rand::thread_rng(),
        }
    }

    pub fn sample(&mut self) -> Vector2f {
        let x = self.range.ind_sample(&mut self.rng);
        let y = self.range.ind_sample(&mut self.rng);
        Vector2f { x, y }
    }
}

fn image_write(filename: &str, resolution: &Vector2u, data: &Vec<u8>) {
    use png::HasParameters;
    use std::fs::File;
    use std::io::BufWriter;
    use std::path::Path;

    let path = Path::new(filename);
    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, resolution.x, resolution.y);
    encoder.set(png::ColorType::RGBA).set(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();

    writer.write_image_data(&data[..]).unwrap();
}
