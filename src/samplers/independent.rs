use super::Sampler;
use crate::geom::vector2::{Vector2f, Vector2u};
use rand::{RngExt, SeedableRng};
use rand_pcg::Pcg64Mcg;

/// Bits reserved for one pixel coordinate in the stream key: 65 536 pixels a side.
const PIXEL_BITS: u32 = 16;

/// Bits reserved for the sample index in the stream key, taking its low end.
///
/// A *reservation*, and that is the point: the field is this wide whatever `samples_ppx` a run
/// asks for, so a pixel's sample 0 has the same key in every run. See
/// `docs/rendu_reproductible.md` §3.
const SAMPLE_INDEX_BITS: u32 = 32;

/// Spreads a seed over the whole 64 bits, so that a small `--seed` is not a small change.
///
/// MurmurHash3's 64-bit finalizer, `fmix64`, with the constants of its reference implementation.
/// It is an avalanche step: flipping one input bit flips about half the output bits.
/// <https://en.wikipedia.org/wiki/MurmurHash>
/// <https://github.com/aappleby/smhasher/blob/master/src/MurmurHash3.cpp>
///
/// The seed cannot be XORed in raw. Neighbouring sample indices give neighbouring keys, so
/// `key(i) ^ key(j) = i ^ j`, and a small seed `s` maps sample `i` onto the key of sample `i ^ s`
/// — the same pixel's own streams, renumbered, hence the same average and the same image.
/// Measured, and worked through, in `docs/rendu_reproductible.md` §5.
///
/// `mix_seed(0) == 0`, so the default seed leaves the key of [1] untouched.
fn mix_seed(mut z: u64) -> u64 {
    z ^= z >> 33;
    z = z.wrapping_mul(0xff51_afd7_ed55_8ccd);
    z ^= z >> 33;
    z = z.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    z ^= z >> 33;
    z
}

/// A sampler drawing numbers independently of one another, from a stream fixed by the pixel and
/// the sample being computed.
///
/// Reference: PBR Book, 4ed, §8.4 — *Independent Sampler*.
/// <https://pbr-book.org/4ed/Sampling_and_Reconstruction/Independent_Sampler>
///
/// The generator's state is a function of `(seed, pixel, sample_index)` and **of nothing else** —
/// not of which thread runs the sample, nor of how many threads there are, nor of the order in
/// which pixels are handed out. A renderer is free to schedule pixels however it likes; it cannot
/// move the numbers.
///
/// # The stream key
///
/// A pixel and one of its samples name exactly one path, so that pair is what the key addresses.
/// The three parts take disjoint bit ranges of a `u64`:
///
///   key = x ⋅ 2⁴⁸ + y ⋅ 2³² + sample_index          [1]
///
/// Disjoint ranges make [1] readable back — hence injective under the bounds asserted below — so
/// two distinct samples cannot land on the same stream. And each field being a fixed *reservation*
/// rather than a share of a total is what keeps a key independent of `samples_ppx`: the arithmetic
/// flattening `pixel_index ⋅ samples_ppx + sample_index` is injective too, but it renumbers every
/// sample when `samples_ppx` changes, so a 4-sample preview would share no path with the 8-sample
/// image it previews. Worked through, with the counter-example, in `docs/rendu_reproductible.md`
/// §2 to §4.
///
/// `mix_seed(seed)` is then XORed in — a bijection, so it permutes the streams without merging any
/// two. The seed is what buys a *different* render of the same scene, which is how one tells a
/// firefly from a bug once two runs are bit-identical; `mix_seed` is what makes `--seed 1`
/// actually different, and its own comment says why. Making *neighbouring* keys yield unrelated
/// generators is a third job again, and it belongs to `SeedableRng::seed_from_u64`, whose
/// documented purpose is exactly that (`docs/rendu_reproductible.md` §5).
///
/// # The engine, and the reach of the promise
///
/// `Pcg64Mcg` is named explicitly because `rand`'s `SmallRng` is free to change algorithm between
/// releases, which would silently rewrite every image on a `cargo update`; its 64-bit output also
/// yields one `f64` per draw. What is promised is bit-identical output for **the same binary on
/// the same machine** — not across machines or compilation profiles, since `sin`, `cos` and `powf`
/// come from the platform's libm. `docs/rendu_reproductible.md` §6.
pub struct IndependentSampler {
    rng: Pcg64Mcg,
}

impl IndependentSampler {
    /// The sampler drawing the numbers of sample `sample_index` of `pixel`.
    pub fn new(seed: u64, pixel: &Vector2u, sample_index: usize) -> Self {
        debug_assert!(
            (pixel.x as u64) < 1 << PIXEL_BITS && (pixel.y as u64) < 1 << PIXEL_BITS,
            "pixel {:?} leaves the {} bits its coordinates are packed into",
            pixel,
            PIXEL_BITS
        );
        debug_assert!(
            (sample_index as u64) < 1 << SAMPLE_INDEX_BITS,
            "sample index {} leaves the {} bits it is packed into",
            sample_index,
            SAMPLE_INDEX_BITS
        );

        let key = ((pixel.x as u64) << (PIXEL_BITS + SAMPLE_INDEX_BITS)) // [1]
            | ((pixel.y as u64) << SAMPLE_INDEX_BITS)
            | (sample_index as u64);

        Self {
            rng: Pcg64Mcg::seed_from_u64(mix_seed(seed) ^ key),
        }
    }
}

impl Sampler for IndependentSampler {
    fn get_1d(&mut self) -> f64 {
        self.rng.random::<f64>()
    }

    fn get_2d(&mut self) -> Vector2f {
        // Bound before building, so that the order of the two draws is on the page rather than in
        // the language's argument evaluation rules.
        let x = self.rng.random::<f64>();
        let y = self.rng.random::<f64>();
        Vector2f::new(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How many draws the distribution tests average over.
    const SAMPLE_COUNT: usize = 100_000;

    fn draw(seed: u64, pixel: &Vector2u, sample_index: usize, count: usize) -> Vec<f64> {
        let mut sampler = IndependentSampler::new(seed, pixel, sample_index);
        (0..count).map(|_| sampler.get_1d()).collect()
    }

    /// The property the whole chantier exists for: the key determines the stream.
    #[test]
    fn test_same_key_gives_the_same_stream() {
        let pixel = Vector2u::new(37, 412);
        assert_eq!(draw(1234, &pixel, 3, 64), draw(1234, &pixel, 3, 64));
    }

    /// Keys one step apart must not walk the same stream. Neighbouring pixels are the case that
    /// matters, since a whole image is made of them and correlation between them is what would
    /// show up as structure rather than as noise.
    #[test]
    fn test_neighbouring_keys_give_different_streams() {
        let reference = draw(0, &Vector2u::new(10, 20), 0, 16);

        assert_ne!(reference, draw(0, &Vector2u::new(11, 20), 0, 16), "pixel x + 1");
        assert_ne!(reference, draw(0, &Vector2u::new(10, 21), 0, 16), "pixel y + 1");
        assert_ne!(reference, draw(0, &Vector2u::new(10, 20), 1, 16), "next sample of the pixel");
        assert_ne!(reference, draw(1, &Vector2u::new(10, 20), 0, 16), "next seed");
    }

    /// A seed exists to give a *different* render, and the first seed anyone types is 1. Without
    /// `mix_seed` this failed exactly there: XOR of a raw seed `s` maps sample `i` onto the key of
    /// sample `i ^ s`, so at 8 samples per pixel seeds 1, 3 and 7 drew the same eight streams as
    /// seed 0 — the same image, and a firefly that never moved however many seeds one tried.
    #[test]
    fn test_a_small_seed_is_not_a_small_change() {
        let pixel = Vector2u::new(0, 0);
        let count = 8;
        let reference = draw(0, &pixel, 0, 1);
        let streams: Vec<f64> = (0..count).flat_map(|i| draw(0, &pixel, i, 1)).collect();

        for seed in 1..=count as u64 {
            let shared = (0..count).flat_map(|i| draw(seed, &pixel, i, 1)).filter(|v| streams.contains(v)).count();
            assert_eq!(shared, 0, "seed {} reuses {} of seed 0's streams", seed, shared);
        }

        // And the default seed is the identity, so [1] is the key it says it is.
        assert_eq!(reference, draw(0, &pixel, 0, 1));
    }

    /// A pdf inverting its cdf needs the half-open interval and nothing wider: `√ξ` and `2πξ` are
    /// defined at 0, and a value of exactly 1 would put a direction on the seam of its domain.
    #[test]
    fn test_values_lie_in_the_unit_interval() {
        let mut sampler = IndependentSampler::new(7, &Vector2u::new(1, 1), 0);
        for _ in 0..SAMPLE_COUNT {
            let Vector2f { x, y } = sampler.get_2d();
            assert!((0.0..1.0).contains(&x), "{} outside [0, 1)", x);
            assert!((0.0..1.0).contains(&y), "{} outside [0, 1)", y);
        }
    }

    /// The first two moments of 𝒰[0,1) are 1/2 and 1/12, and an estimator that missed either would
    /// bias every pdf built on top of it.
    ///
    /// The tolerances are the standard errors of the two estimators, times five. For N draws:
    ///
    ///   Var[mean] = σ²/N = 1/(12N)                  ⟹  se ≈ 9.1 ⋅ 10⁻⁴ at N = 10⁵
    ///   Var[s²]   = (μ₄ − σ⁴)/N = (1/80 − 1/144)/N  ⟹  se ≈ 2.4 ⋅ 10⁻⁴ at N = 10⁵
    ///
    /// Five standard errors would be a 6 ⋅ 10⁻⁷ chance of a false failure on a *random* stream —
    /// but the stream here is seeded, so the test either passes or fails, always the same way.
    /// That is the point of the chantier applied to its own test.
    ///
    /// This stream lands at 2.2 and 0.9 standard errors respectively: the bounds hold with room,
    /// and they hold on a draw that is itself unremarkable rather than a lucky one.
    #[test]
    fn test_first_two_moments_match_the_uniform_distribution() {
        let values = draw(99, &Vector2u::new(3, 4), 0, SAMPLE_COUNT);

        let mean = values.iter().sum::<f64>() / SAMPLE_COUNT as f64;
        let variance = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / SAMPLE_COUNT as f64;

        assert!((mean - 0.5).abs() < 0.005, "mean {} is not 1/2", mean);
        assert!((variance - 1.0 / 12.0).abs() < 0.002, "variance {} is not 1/12", variance);
    }
}
