use crate::cameras::Camera;
use crate::config::Config;
use crate::geom::bounds2::Bounds2;
use crate::geom::vector2::{Vector2, Vector2f, Vector2u};
use crate::integrators::Integrator;
use crate::progress::ProgressBar;
use crate::samplers::{IndependentSampler, Sampler};
use crate::scene::Scene;
use crate::spectrum::Spectrum;
use std::f64;
use std::sync::mpsc;
use std::thread::{self, ScopedJoinHandle};

enum Request {
    Compute { coords: Vector2u },
    Quit,
}

struct Response {
    coords: Vector2u,
    spectrum: Spectrum,
}

pub fn render(config: &Config, scene: &Scene, camera: &dyn Camera, integrator: &dyn Integrator) {
    let image_width = config.output_width as u32;
    let image_height = config.output_height as u32;

    let mut pixels: Vec<u8> = Vec::new();
    pixels.resize((image_width * image_height * 4) as usize, 0);

    let resolution = Vector2u::new(image_width, image_height);
    let patch = Bounds2::new(&Vector2::new(0, 0), &resolution);
    let pixel_count = image_width * image_height as u32;

    thread::scope(|s| {
        let (upstream_tx, upstream_rx) = mpsc::channel();
        let mut handles: Vec<ScopedJoinHandle<()>> = Vec::new();
        let mut senders: Vec<mpsc::Sender<Request>> = Vec::new();

        for _ in 0..(config.threads) {
            let (tx, rx): (mpsc::Sender<Request>, mpsc::Receiver<Request>) = mpsc::channel();
            let upstream_tx = upstream_tx.clone();

            let handle = s.spawn(move || {
                loop {
                    match rx.recv().unwrap() {
                        Request::Quit => break,

                        Request::Compute { coords } => {
                            let spectrum = compute_pixel(config, integrator, coords, camera, scene);
                            let response = Response { coords, spectrum };
                            upstream_tx.send(response).unwrap();
                        }
                    }
                }
            });
            handles.push(handle);
            senders.push(tx);
        }

        let mut pixel_iter = patch.to_iter();
        let mut pixel_computed = 0;
        let mut tid = 0;
        let mut progress = ProgressBar::new(pixel_count as usize);
        while pixel_computed < image_width * image_height {
            match pixel_iter.next() {
                None => {}
                Some(coords) => {
                    senders[tid].send(Request::Compute { coords }).unwrap();
                    tid = (tid + 1) % config.threads;
                }
            }

            match upstream_rx.try_recv() {
                Ok(Response { coords, spectrum }) => {
                    let mut res = spectrum;
                    res.gamma_correct();
                    let sample = res.to_rgb();
                    let pixel_index = ((coords.y * image_width + coords.x) * 4) as usize;
                    pixels[pixel_index] = sample[0];
                    pixels[pixel_index + 1] = sample[1];
                    pixels[pixel_index + 2] = sample[2];
                    pixels[pixel_index + 3] = sample[3];
                    pixel_computed = pixel_computed + 1;

                    progress.advance();
                }
                Err(_) => {}
            }
        }
        progress.finish();

        for sender in &senders {
            sender.send(Request::Quit).unwrap();
        }

        handles.drain(..).for_each(|handle| {
            handle.join().unwrap();
            ()
        });
    });

    image_write(&config.output_filename, &resolution, &pixels);
}

fn compute_pixel(config: &Config, integrator: &dyn Integrator, pixel_coords: Vector2<u32>, camera: &dyn Camera, scene: &Scene) -> Spectrum {
    let mut res = Spectrum::new(0.0, 0.0, 0.0);
    let pixel_origin = Vector2f::from(pixel_coords);
    for sample_index in 0..config.samples_ppx {
        // One sampler per sample, keyed on the pixel and the index — so the numbers this path
        // draws depend on neither the thread nor the order pixels are handed out. That is what
        // makes a run at one thread and a run at eight produce the same image.
        let mut sampler = IndependentSampler::new(config.seed, &pixel_coords, sample_index);
        let p_film = pixel_origin + sampler.get_2d();
        let ray = camera.get_ray(&p_film, &mut sampler);
        res += integrator.li(&ray, &scene, config.max_depth, config.near, config.far, &mut sampler);
    }
    res * (1.0 / (config.samples_ppx as f64))
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
