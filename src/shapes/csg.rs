use crate::geom::intersectable::Intersectable;
use crate::geom::transform::Transform;

pub struct Elem {
    pub shape: Box<dyn Intersectable>,
    pub transform: Box<Transform>
}

mod intersection;
mod substraction;
mod union;

pub use self::intersection::Intersection;
pub use self::substraction::Substraction;
pub use self::union::Union;
