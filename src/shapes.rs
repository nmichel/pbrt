use crate::geom::aabound::AABound;
use crate::geom::intersectable::Intersectable;

pub trait Shape: Intersectable + AABound {}

mod aabox;
mod cylinder;
mod plane;
mod rectangle;
mod sphere;
mod triangle;

pub mod csg;

pub use self::aabox::AABox;
pub use self::cylinder::Cylinder;
pub use self::plane::Plane;
pub use self::rectangle::Rectangle;
pub use self::sphere::Sphere;
pub use self::triangle::Triangle;
