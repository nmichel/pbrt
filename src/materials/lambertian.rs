use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::spectrum::Spectrum;
use crate::textures::*;
use crate::utils::random_in_unit_sphere;
use std::sync::Arc;
use super::Material;

pub struct Lambertian {
    albedo: Arc<dyn Texture>
}

impl Lambertian {
    pub fn new(albedo: Arc<dyn Texture>) -> Self {
        Self { albedo }
    }
}

impl Material for Lambertian {
    fn scatter(&self, _ray: &Ray, interaction: &Interaction) -> Option<(Spectrum, Ray)> {
        let Interaction { ref intersection, .. } = interaction;
        let Intersection { ref p, ref n, .. } = intersection;
        let scatter_dir: Vector3f = *n + random_in_unit_sphere();
        let scatter_origin = *p + scatter_dir * 0.001;
        let scattered_ray = Ray::new(&scatter_origin, &scatter_dir);
        let attenuation = self.albedo.shade(intersection);
        return Some((attenuation, scattered_ray));
    }
}
