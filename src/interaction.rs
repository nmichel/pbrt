use super::geom::intersectable::Intersection;
use super::materials::Material;
use std::sync::Arc;

pub struct Interaction {
    /// Geometry of the interaction
    pub intersection: Intersection,

    /// Material at intersection
    pub material: Arc<dyn Material>,
}
