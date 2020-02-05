use crate::geom::intersectable::Intersectable;
use crate::geom::transform::Transform;

pub struct Elem {
    pub shape: Box<Intersectable>,
    pub transform: Box<Transform>
}

mod intersection;
mod union;

pub use self::union::Union;
pub use self::intersection::CSGIntersection;
