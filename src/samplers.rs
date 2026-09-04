use crate::geom::vector2::Vector2f;

/// The source of every random number a render draws.
///
/// Reference: PBR Book, 4ed, §8.3 — *Sampling Interface*.
/// <https://pbr-book.org/4ed/Sampling_and_Reconstruction/Sampling_Interface>
///
/// **What a sampler is for.** Path tracing needs an unbounded supply of numbers in [0, 1): one to
/// pick a light, two to pick a direction, two more at the next bounce. This trait is the single
/// door they come through, so that *where* they come from is decided in one place — by whoever
/// builds the sampler — instead of by each material and each pdf reaching for a global generator.
///
/// **Why 1D and 2D and nothing else.** These are the shapes the renderer actually consumes. A
/// direction, a point on a lens, a position inside a pixel are all *pairs*, and they are pairs in
/// a way that matters: a sampler that spreads its samples deliberately must treat a pair as one
/// two-dimensional quantity, not as two unrelated numbers. Asking for `get_2d` rather than calling
/// `get_1d` twice is what leaves that door open. `IndependentSampler` makes no use of the
/// distinction, and that is fine — the interface records the caller's intent, not the current
/// implementation's needs.
///
/// **What the trait deliberately does not say.** Nothing about seeds, pixels, or which sample of a
/// pixel is being drawn. Those are how a *particular* sampler is built, so they belong to its
/// constructor: a caller holding a `&mut dyn Sampler` asks for numbers and learns nothing else.
/// That is what lets a different implementation be dropped in at the one line that builds it.
pub trait Sampler {
    /// The next number, uniform over [0, 1).
    fn get_1d(&mut self) -> f64;

    /// The next point, uniform over [0, 1)².
    fn get_2d(&mut self) -> Vector2f;
}

mod independent;

pub use self::independent::IndependentSampler;
