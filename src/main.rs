use pbrt::config::Config;
use pbrt::geom::matrix4::Matrix4;
use pbrt::geom::intersectable::Intersectable;
use pbrt::geom::ray::Ray;
use pbrt::geom::sphere::Sphere;
use pbrt::geom::vector3::Vector3f;
use pbrt::scene::Scene;
use std::env;
use std::f64;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(1);        
    });

    let mut scene = Scene::new();
    scene
        .add(Box::new(Sphere::new(Vector3f::new(0., 0., 0.), 1.)))
        .add(Box::new(Sphere::new(Vector3f::new(0., 10., 0.), 1.)));

    let image_width: u64 = 1920;
    let image_height: u64 = 1024;
    let image_aspect_ratio = (image_width as f64) / (image_height as f64);

    let fov = 90.0 * f64::consts::PI / 180.0;

    let screen_p_min_x;
    let screen_p_max_x;
    let screen_p_min_y;
    let screen_p_max_y;

    if image_aspect_ratio > 1.0 {
        screen_p_min_x = -image_aspect_ratio;
        screen_p_max_x = image_aspect_ratio;
        screen_p_min_y = -1.0;
        screen_p_max_y = 1.0;
    } else {
        screen_p_min_x = -1.0;
        screen_p_max_x = 1.0;
        screen_p_min_y = -1.0 / image_aspect_ratio;
        screen_p_max_y = 1.0 / image_aspect_ratio;
    }

    println!("ratio {}", image_aspect_ratio);
    println!("screen {} {} {} {}", screen_p_min_x, screen_p_max_x, screen_p_min_y, screen_p_max_y);

    let trans =
        Matrix4::perspective(fov, 0.01, 1000.0).inverse() *
        Matrix4::translation(screen_p_min_x, screen_p_max_y, 0.0) *
        Matrix4::scale(screen_p_max_x - screen_p_min_x, screen_p_min_y - screen_p_max_y, 1.0) *
        Matrix4::scale(1.0 / (image_width as f64), 1.0 / (image_height as f64), 1.0);

    let mut pixels:Vec<u8> = Vec::new();
    for pixel_y in 0..image_height {
        let r = (((pixel_y+1) as f64 / image_height as f64) * 255.) as u8;
        for pixel_x in 0..image_width {
            let camera_vector = &trans * &Vector3f::new(pixel_x as f64, pixel_y as f64, 0.0);
            println!("[{}, {}] => {}", pixel_x, pixel_y, camera_vector);

            let g = (((pixel_x+1) as f64 / image_width as f64) * 255.) as u8;
            let mut sample = vec![r, g, 0, 255];

            let ray = Ray::new();
            scene.intersect(&ray).iter().for_each(|item| {
                println!("Intersection {}", item);
            });

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
/*
fn build_camera() {
    let focal_length = 35.0; // mm
    let film_apperture_height = 0.735; // inch
    let film_apperture_width = 0.980; // inch
    const inch_to_mm: f64 = 25.4;
    let near = 0.1;
    let far = 1000.0;

    let film_aspect_ratio = film_apperture_width / film_apperture_height; 
    let top = ((film_apperture_height * inch_to_mm / 2.0) / focal_length) * near; 
    let right = top * film_aspect_ratio; 

    enum FitResolutionGate { kFill = 0, kOverscan }; 
    FitResolutionGate fitFilm = kOverscan; 

    float xscale = 1; 
    float yscale = 1; 
 
    switch (fitFilm) { 
        default: 
        case kFill: 
            if (filmAspectRatio > deviceAspectRatio) { 
                xscale = deviceAspectRatio / filmAspectRatio; 
            } 
            else { 
                yscale = filmAspectRatio / deviceAspectRatio; 
            } 
            break; 
        case kOverscan: 
            if (filmAspectRatio > deviceAspectRatio) { 
                yscale = filmAspectRatio / deviceAspectRatio; 
            } 
            else { 
                xscale = deviceAspectRatio / filmAspectRatio; 
            } 
            break; 
    } 
 
    right *= xscale; 
    top *= yscale; 
 
    let bottom = -top; 
    let left = -right;
}

fn screen_to_NDC(screen_coord, l, r, t, b) {
    Vec2f pNDC; 
    pNDC.x = (screen_coord.x - l) / (r - t); // Translate then rescale
    pNDC.y = (screen_coord.y - b) / (t - b);     
}

fn NDC_to_raster(ndc_coord, image_width, image_height) {
    Vec2i pRaster;
    pRaster.x = (int)(ndc_coord.x * image_width); 
    pRaster.y = (int)((1 - ndc_coord.y) * image_height); 
}
*/