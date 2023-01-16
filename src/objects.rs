use crate::geom::aabound::AABound;
use crate::geom::intersectable::Intersectable;
use crate::geom::ray::Ray;
use crate::interaction::Interaction;

pub trait Object: Intersectable + AABound {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Interaction>;
}

mod compound;
mod simple;
mod transformed;

pub use self::compound::Compound;
pub use self::simple::Simple;
pub use self::transformed::Transformed;
