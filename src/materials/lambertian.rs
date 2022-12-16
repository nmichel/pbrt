use super::Material;
use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::spectrum::Spectrum;
use crate::textures::*;
use crate::utils::random_unit_vector;
use std::sync::Arc;

pub struct Lambertian {
    albedo: Arc<dyn Texture>,
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

        let mut scatter_direction: Vector3f = *n + random_unit_vector();
        // Catch degenerate scatter direction
        scatter_direction.normalize();
        if scatter_direction.near_zero() {
            scatter_direction = *n;
        }

        let shift_avoid_acne = n * 0.001;
        let scattered_ray = Ray::new(&(p + &shift_avoid_acne), &scatter_direction);
        let attenuation = self.albedo.shade(intersection);
        return Some((attenuation, scattered_ray));
    }
}
