use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::interaction::Interaction;
use crate::spectrum::Spectrum;
use crate::textures::*;
use crate::utils::random_in_unit_sphere;
use std::sync::Arc;
use super::material::Material;

pub struct Lambertian {
    albedo: Arc<Texture>
}

impl Lambertian {
    pub fn new(albedo: Arc<Texture>) -> Self {
        Self { albedo }
    }
}

impl Material for Lambertian {
    fn scatter(&self, _ray: &Ray, interaction: &Interaction) -> Option<(Spectrum, Ray)> {
        let Interaction { ref intersection, .. } = interaction;
        let Intersection { ref p, ref n, .. } = intersection;
        let scatter_dir = n + &random_in_unit_sphere();
        let scattered_ray = Ray::new(p, &scatter_dir);
        let attenuation = self.albedo.shade(intersection);
        return Some((attenuation, scattered_ray));
    }
}
