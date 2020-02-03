use crate::geom::intersectable::Intersection;
use crate::spectrum::Spectrum;

pub trait Texture : Send + Sync {
    fn shade(&self, interaction: &Intersection) -> Spectrum;
}
