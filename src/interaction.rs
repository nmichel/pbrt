use super::geom::intersectable::Intersection;
use super::materials::material::Material;

pub struct Interaction<'a> {
    /// Geometry of the interaction
    pub intersection: Intersection,

    /// Material at intersection
    pub  material: &'a (Material + 'a)
}
