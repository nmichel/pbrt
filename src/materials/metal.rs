use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::spectrum::Spectrum;
use crate::textures::*;
use crate::utils::random_in_unit_sphere;
use std::sync::Arc;
use super::material::Material;

pub struct Metal {
    fuzz: f64,
    albedo: Arc<Texture>
}

impl Metal {
    pub fn new(fuzz: f64, albedo: Arc<Texture>) -> Self {
        Self { fuzz, albedo }
    }
}

impl Material for Metal {
    fn scatter(&self, _ray: &Ray, interaction: &Interaction) -> Option<(Spectrum, Ray)> {
        let Interaction { ref intersection, .. } = interaction;
        let Intersection { ref p, ref n, ref wo, .. } = intersection;

        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let local_reflected = Vector3f::new(-local_wo.x, -local_wo.y, local_wo.z);
        let mut local_target = local_reflected + random_in_unit_sphere() * self.fuzz;
        local_target.normalize();

        if local_target.z > 0.0 { // <=> dot(local_target, n)
            let target = intersection.local_to_world(&local_target);
            let scattered_ray = Ray::new(p, &target);
            let attenuation = self.albedo.shade(intersection);
            Some((attenuation, scattered_ray))
        }
        else {
            None
        }
    }
}
