use crate::geom::ray::Ray;

pub trait Camera : Send + Sync {
  /// Returns the `Ray` passing through pixel at coordinates `pixel`
  fn get_ray(&self, pixel_x: f64, pixel_y: f64) -> Ray;
}

mod pin_hole;
mod thin_lens;

pub use pin_hole::PinHoleCamera;
pub use thin_lens::ThinLensCamera;
