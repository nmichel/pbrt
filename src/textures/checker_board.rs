use crate::geom::intersectable::Intersection;
use crate::spectrum::Spectrum;
use super::texture::Texture;

pub struct CheckerBoard {
    c1: Spectrum,
    c2: Spectrum,
    scale: f64
}

impl CheckerBoard {
    pub fn new(c1: Spectrum, c2: Spectrum, scale: f64) -> Self {
        Self { c1, c2, scale }
    }
}

impl Texture for CheckerBoard {
    fn shade(&self, interaction: &Intersection) -> Spectrum {
        let scale_u = (interaction.u.abs() * self.scale) % 1.0;
        let scale_v = (interaction.v.abs() * self.scale) % 1.0;
        let use_color = (scale_u < 0.5 && scale_v < 0.5) || (scale_u >= 0.5 && scale_v >= 0.5);
        let positive_uv_prod = (interaction.u.signum() * interaction.v.signum()) >= 0.0;
        if use_color ^ positive_uv_prod {
            self.c1
        }
        else {
            self.c2
        }
    }
}

