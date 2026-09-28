//! Reduces a rendered image to numbers, and compares two of them.
//!
//! ```text
//! cargo run --release --bin image_stats -- render.png
//! cargo run --release --bin image_stats -- render.png reference.png
//! ```
//!
//! # Why a tool and not a test
//!
//! Like [`bvh_stats`](bvh_stats), it asserts nothing. There is no reference value for the mean of
//! an image; there is only the other run's, and what the comparison means is a matter of what
//! changed between them. The figures belong in the commit message of a change that moves an image,
//! and in `docs/` when they settle an arbitration.
//!
//! What it is *for* is the two measurements the lighting work needs and `cmp` cannot give. `cmp`
//! answers one question — are these the same bytes — which settles a refactor and nothing else. A
//! change that is *meant* to move the image needs to know **by how much**, and in which direction.
//!
//! - **The means** say whether two estimators agree. A missing cosine, an inverse square applied
//!   twice, a measure left unconverted: all of them scale an image without changing its shape, so
//!   the eye does not catch them and a second estimator does.
//! - **The root mean square against a reference** says how *noisy* an image is, which is the
//!   quantity an importance-sampling change is supposed to move. Two images with the same mean and
//!   very different noise are the before and after of that kind of work.
//!
//! # What these numbers are, and the trap in them
//!
//! **They are read off the encoded image**, after gamma and after clamping to 0-255. That is what a
//! viewer sees, and it is deliberately the thing measured — but it is not a linear radiometric
//! quantity, and two consequences follow that have already cost this project a wrong conclusion.
//!
//! **Noise raises the mean.** Gamma encoding is concave, so by Jensen's inequality the mean of the
//! encoded image sits above the encoding of the mean, and the gap grows with the variance. A noisy
//! render is therefore *brighter on average* than the converged one it is heading towards, at equal
//! true radiance. Measured on `test_files/cornell_box_exact.stage` under `naive`: 59.54 at 64 paths
//! per pixel, 74.62 at 4096, 74.79 at 16384. **Comparing two estimators by their means proves
//! nothing until the slower one has been shown to stop moving** — which takes a ladder of sample
//! counts, not a single render.
//!
//! **A clamped pixel has lost its energy**, and no amount of averaging brings it back. That is why
//! the saturated count is printed beside the mean: it is the share of the image the mean cannot
//! speak for. An image with many saturated pixels is one whose mean understates the light in it.
//!
//! # Why a reference image is never committed
//!
//! A render replays from its seed, bit for bit, whatever the thread count and the renderer
//! (`docs/rendu_reproductible.md`). A reference is therefore a command line, not a file, and the
//! second argument here is an image one regenerates rather than one the repository stores.

use std::fs::File;
use std::{env, process};

/// The channels a mean is reported for. The renderer writes RGBA8, and the alpha channel carries
/// no light — it is a constant the encoder needs, so averaging it would only dilute the figures.
const CHANNEL_COUNT: usize = 3;
const CHANNEL_NAMES: [&str; CHANNEL_COUNT] = ["R", "G", "B"];

/// Bytes per pixel in the decoded buffer.
const BYTES_PER_PIXEL: usize = 4;

/// The value a channel clamps at. A pixel sitting here was cut off by the encoding, not measured.
const SATURATED: u8 = 255;

/// A decoded image, kept as the bytes the file holds rather than as floats: every figure this tool
/// prints is a statement about the encoded image, and converting would invite the reader to forget
/// it.
struct Image {
    path: String,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Image {
    fn read(path: &str) -> Result<Self, String> {
        let file = File::open(path).map_err(|err| format!("cannot open '{}': {}", path, err))?;
        let decoder = png::Decoder::new(file);
        let (info, mut reader) = decoder.read_info().map_err(|err| format!("cannot decode '{}': {}", path, err))?;

        if info.color_type != png::ColorType::RGBA || info.bit_depth != png::BitDepth::Eight {
            return Err(format!(
                "'{}' is {:?}/{:?}; this tool reads the RGBA8 the renderer writes",
                path, info.color_type, info.bit_depth
            ));
        }

        let mut pixels = vec![0; info.buffer_size()];
        reader
            .next_frame(&mut pixels)
            .map_err(|err| format!("cannot read the pixels of '{}': {}", path, err))?;

        Ok(Self {
            path: path.to_string(),
            width: info.width,
            height: info.height,
            pixels,
        })
    }

    fn pixel_count(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    /// Mean value of each channel, over every pixel.
    fn means(&self) -> [f64; CHANNEL_COUNT] {
        let mut sums = [0u64; CHANNEL_COUNT];
        for pixel in self.pixels.chunks_exact(BYTES_PER_PIXEL) {
            for channel in 0..CHANNEL_COUNT {
                sums[channel] += u64::from(pixel[channel]);
            }
        }

        let count = self.pixel_count() as f64;
        let mut means = [0.0; CHANNEL_COUNT];
        for channel in 0..CHANNEL_COUNT {
            means[channel] = sums[channel] as f64 / count;
        }
        means
    }

    /// How many pixels have that channel clamped at [`SATURATED`].
    fn saturated_counts(&self) -> [usize; CHANNEL_COUNT] {
        let mut counts = [0; CHANNEL_COUNT];
        for pixel in self.pixels.chunks_exact(BYTES_PER_PIXEL) {
            for channel in 0..CHANNEL_COUNT {
                if pixel[channel] == SATURATED {
                    counts[channel] += 1;
                }
            }
        }
        counts
    }
}

/// How far apart two images are.
struct Difference {
    /// Root mean square of the per-channel differences, over the three colour channels of every
    /// pixel. One number for the whole image, in levels of 0-255.
    rms: f64,

    /// Largest difference any single channel shows, and where it is. A root mean square is an
    /// average and hides a handful of wild pixels; a firefly lives exactly there.
    worst: u8,
    worst_at: (u32, u32),

    /// Whether the two files hold the same bytes, which is a stronger statement than a zero root
    /// mean square and is the one a refactor has to make.
    identical: bool,
}

fn compare(image: &Image, reference: &Image) -> Result<Difference, String> {
    if image.width != reference.width || image.height != reference.height {
        return Err(format!(
            "'{}' is {}×{} and '{}' is {}×{}; there is nothing to compare",
            image.path, image.width, image.height, reference.path, reference.width, reference.height
        ));
    }

    let mut sum_of_squares = 0.0;
    let mut worst = 0;
    let mut worst_at = (0, 0);

    for (index, (pixel, reference_pixel)) in image
        .pixels
        .chunks_exact(BYTES_PER_PIXEL)
        .zip(reference.pixels.chunks_exact(BYTES_PER_PIXEL))
        .enumerate()
    {
        for channel in 0..CHANNEL_COUNT {
            // In u8 arithmetic the subtraction of the smaller from the larger is the distance, and
            // it cannot overflow the way `a - b` would.
            let difference = pixel[channel].max(reference_pixel[channel]) - pixel[channel].min(reference_pixel[channel]);
            sum_of_squares += f64::from(difference) * f64::from(difference);
            if difference > worst {
                worst = difference;
                worst_at = ((index as u32) % image.width, (index as u32) / image.width);
            }
        }
    }

    let sample_count = (image.pixel_count() * CHANNEL_COUNT) as f64;

    Ok(Difference {
        rms: (sum_of_squares / sample_count).sqrt(),
        worst,
        worst_at,
        identical: image.pixels == reference.pixels,
    })
}

fn print_image(image: &Image) {
    let means = image.means();
    let saturated = image.saturated_counts();
    let count = image.pixel_count() as f64;

    println!("{}", image.path);
    println!("  {} × {}, {} pixels", image.width, image.height, image.pixel_count());
    for channel in 0..CHANNEL_COUNT {
        println!(
            "  {:<4} mean {:8.2}   saturated {:>8} ({:.2} %)",
            CHANNEL_NAMES[channel],
            means[channel],
            saturated[channel],
            100.0 * saturated[channel] as f64 / count
        );
    }
}

fn print_difference(image: &Image, reference: &Image, difference: &Difference) {
    let means = image.means();
    let reference_means = reference.means();

    println!("against {}", reference.path);
    for channel in 0..CHANNEL_COUNT {
        println!(
            "  {:<4} {:8.2} − {:8.2} = {:+8.2}",
            CHANNEL_NAMES[channel],
            means[channel],
            reference_means[channel],
            means[channel] - reference_means[channel]
        );
    }
    println!("  rms                        {:8.2}", difference.rms);
    println!(
        "  worst pixel                {:8} at ({}, {})",
        difference.worst, difference.worst_at.0, difference.worst_at.1
    );
    if difference.identical {
        println!("  the two files hold the same bytes");
    }
}

fn main() {
    let paths: Vec<String> = env::args().skip(1).collect();

    let (image_path, reference_path) = match paths.as_slice() {
        [image] => (image.as_str(), None),
        [image, reference] => (image.as_str(), Some(reference.as_str())),
        _ => {
            eprintln!("usage: image_stats <render.png> [reference.png]");
            process::exit(2);
        }
    };

    let run = || -> Result<(), String> {
        let image = Image::read(image_path)?;
        print_image(&image);

        if let Some(reference_path) = reference_path {
            let reference = Image::read(reference_path)?;
            println!();
            let difference = compare(&image, &reference)?;
            print_difference(&image, &reference, &difference);
        }

        Ok(())
    };

    if let Err(message) = run() {
        eprintln!("image_stats: {}", message);
        process::exit(1);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// An image of `width × height` pixels, every one of them the same colour.
    fn flat(width: u32, height: u32, colour: [u8; CHANNEL_COUNT]) -> Image {
        let mut pixels = Vec::new();
        for _ in 0..(width as usize) * (height as usize) {
            pixels.extend_from_slice(&colour);
            pixels.push(SATURATED); // alpha, which no figure reads
        }
        Image {
            path: "flat".to_string(),
            width,
            height,
            pixels,
        }
    }

    /// The mean of a flat image is its own colour, and the alpha channel does not reach it.
    ///
    /// Alpha is the trap this pins: it is written at 255 on every pixel, so a mean taken over four
    /// channels instead of three would read high, and read *differently* high on a dark image than
    /// on a bright one. The colour here is deliberately far from 255.
    #[test]
    fn test_the_mean_of_a_flat_image_is_its_colour() {
        let image = flat(4, 3, [10, 20, 30]);

        assert_eq!(image.means(), [10.0, 20.0, 30.0]);
    }

    /// Half the pixels at one value and half at another put the mean exactly between them.
    #[test]
    fn test_the_mean_is_taken_over_every_pixel() {
        let mut image = flat(2, 1, [0, 0, 0]);
        image.pixels[BYTES_PER_PIXEL..].copy_from_slice(&[100, 200, 40, SATURATED]);

        assert_eq!(image.means(), [50.0, 100.0, 20.0]);
    }

    /// Only the channels that clamp are counted, and only where they clamp.
    #[test]
    fn test_a_clamped_channel_is_counted_as_saturated() {
        let mut image = flat(2, 1, [SATURATED, 0, 0]);
        image.pixels[BYTES_PER_PIXEL..].copy_from_slice(&[SATURATED, SATURATED, 0, SATURATED]);

        assert_eq!(image.saturated_counts(), [2, 1, 0]);
    }

    /// An image compared with itself is at distance zero, and says so twice.
    #[test]
    fn test_an_image_does_not_differ_from_itself() {
        let image = flat(3, 2, [17, 200, 99]);

        let difference = compare(&image, &image).unwrap();

        assert_eq!(difference.rms, 0.0);
        assert_eq!(difference.worst, 0);
        assert!(difference.identical);
    }

    /// A constant offset on one channel of three gives a root mean square of `offset / √3`.
    ///
    /// Taken over the three colour channels and not over one: the whole image differs by 30 on
    /// green and by nothing elsewhere, so `√((0² + 30² + 0²) / 3)` = 17.32. A root mean square
    /// computed per channel would answer 30, and a difference spread over three channels would
    /// then read the same as one concentrated in a single one.
    #[test]
    fn test_the_root_mean_square_spans_the_three_channels() {
        let image = flat(4, 4, [60, 90, 120]);
        let reference = flat(4, 4, [60, 60, 120]);

        let difference = compare(&image, &reference).unwrap();

        assert!((difference.rms - 30.0 / 3.0_f64.sqrt()).abs() < 1e-12);
        assert_eq!(difference.worst, 30);
        assert!(!difference.identical);
    }

    /// The worst pixel is found wherever it is, and reported at its own coordinates.
    ///
    /// The image is wider than it is tall so that a transposed index — row for column — lands
    /// outside the image rather than on a plausible pixel.
    #[test]
    fn test_the_worst_pixel_is_reported_where_it_sits() {
        let image = flat(5, 2, [10, 10, 10]);
        let mut reference = flat(5, 2, [10, 10, 10]);

        // Pixel (3, 1), so index 1 × 5 + 3 = 8.
        let offset = 8 * BYTES_PER_PIXEL;
        reference.pixels[offset..offset + BYTES_PER_PIXEL].copy_from_slice(&[10, 10, 90, SATURATED]);

        let difference = compare(&image, &reference).unwrap();

        assert_eq!(difference.worst, 80);
        assert_eq!(difference.worst_at, (3, 1));
    }

    /// Two images of different sizes are not compared at all, rather than compared over the part
    /// they share: a render at another resolution is a different measurement, not a partial one.
    #[test]
    fn test_images_of_different_sizes_are_refused() {
        let image = flat(4, 4, [0, 0, 0]);
        let reference = flat(4, 5, [0, 0, 0]);

        assert!(compare(&image, &reference).is_err());
    }
}
