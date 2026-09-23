use crate::geom::surface_point::SurfacePoint;
use crate::spectrum::Spectrum;

/// A spectral value at a point on a surface.
///
/// The argument is a [`SurfacePoint`] and not an `Intersection`, because that is the whole of what
/// a texture is a function of: what it returns cannot depend on how the point was reached, so a
/// texture handed a distance and a view direction could only be tempted to read them.
pub trait Texture: Send + Sync {
    fn shade(&self, surface_point: &SurfacePoint) -> Spectrum;
}

mod checker_board;
mod plain_color;

pub use self::checker_board::CheckerBoard;
pub use self::plain_color::PlainColor;
