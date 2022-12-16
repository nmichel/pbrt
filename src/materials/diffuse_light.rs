use super::Material;
use crate::geom::ray::Ray;
use crate::interaction::Interaction;
use crate::spectrum::Spectrum;
use crate::textures::*;
use std::sync::Arc;

pub struct DiffuseLight {
    emitted: Arc<dyn Texture>,
}

impl DiffuseLight {
    pub fn new(emitted: Arc<dyn Texture>) -> Self {
        Self { emitted }
    }
}

impl Material for DiffuseLight {
    fn emit(&self, _ray: &Ray, interaction: &Interaction) -> Option<Spectrum> {
        Some(self.emitted.shade(&interaction.intersection))
    }
}
