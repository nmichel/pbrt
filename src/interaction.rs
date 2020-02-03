use super::geom::intersectable::Intersection;
use super::materials::Material;

pub struct Interaction<'a> {
    /// Geometry of the interaction
    pub intersection: Intersection,

    /// Material at intersection
    pub  material: &'a Material
}
