use super::Texture;
use crate::geom::intersectable::Intersection;
use crate::spectrum::Spectrum;

pub struct PlainColor {
    c: Spectrum,
}

impl PlainColor {
    pub fn new(c: Spectrum) -> Self {
        Self { c }
    }
}

impl Texture for PlainColor {
    fn shade(&self, _interaction: &Intersection) -> Spectrum {
        self.c
    }
}
