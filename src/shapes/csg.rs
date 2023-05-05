use std::sync::Arc;

use crate::geom::transform::Transform;

use super::Shape;

pub struct Elem {
    pub shape: Arc<dyn Shape>,
    pub transform: Box<Transform>,
}

mod intersection;
mod substraction;
mod union;

pub use self::intersection::Intersection;
pub use self::substraction::Substraction;
pub use self::union::Union;
