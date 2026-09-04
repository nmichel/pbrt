use crate::geom::ray::Ray;
use crate::geom::vector2::Vector2f;
use crate::samplers::Sampler;

pub trait Camera: Send + Sync {
    /// The `Ray` leaving the camera through `p_film`.
    ///
    /// **`p_film`** is a continuous position on the film, in raster coordinates: its integer part
    /// names the pixel, its fractional part places the ray inside it. It belongs to the caller
    /// because the *film* does — a renderer owns pixels, decides how they are sampled, and a camera
    /// has no opinion on either.
    ///
    /// **`sampler`** is where the optics take whatever else they need. A thin lens draws a point on
    /// its aperture, a pinhole draws nothing, and the caller is told neither: how many numbers an
    /// optical system consumes is a property of that system. Handing every camera a fixed set of
    /// pre-drawn quantities would put that property in the caller instead, and make a pinhole
    /// render pay for an aperture it does not have.
    fn get_ray(&self, p_film: &Vector2f, sampler: &mut dyn Sampler) -> Ray;
}

mod pin_hole;
mod thin_lens;

pub use pin_hole::PinHoleCamera;
pub use thin_lens::ThinLensCamera;
