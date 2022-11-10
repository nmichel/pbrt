use crate::geom::intersectable::Intersection;
use crate::spectrum::Spectrum;

pub trait Texture : Send + Sync {
    fn shade(&self, interaction: &Intersection) -> Spectrum;
}

mod checker_board;
mod plain_color;

pub use self::checker_board::CheckerBoard;
pub use self::plain_color::PlainColor;
