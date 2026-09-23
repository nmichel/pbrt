use super::Texture;
use crate::geom::surface_point::SurfacePoint;
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
    fn shade(&self, _surface_point: &SurfacePoint) -> Spectrum {
        self.c
    }
}
