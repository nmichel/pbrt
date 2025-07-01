use crate::geom::aabound::AABound;
use crate::geom::intersectable::Intersectable;

pub trait Shape: Intersectable + AABound {}

mod aabox;
mod bvh;
mod cylinder;
mod plane;
mod rectangle;
mod sphere;
mod triangle;
pub(crate) mod triangle_mesh;

pub mod csg;

pub use self::aabox::AABox;
pub use self::cylinder::Cylinder;
pub use self::plane::Plane;
pub use self::rectangle::Rectangle;
pub use self::sphere::Sphere;
pub use self::triangle::Triangle;
pub use self::triangle_mesh::TriangleMesh;
