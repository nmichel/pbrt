use super::Material;
use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::spectrum::Spectrum;
use crate::textures::*;
use crate::utils::random_in_unit_sphere;
use std::sync::Arc;

pub struct Metal {
    fuzz: f64,
    albedo: Arc<dyn Texture>,
}

impl Metal {
    pub fn new(fuzz: f64, albedo: Arc<dyn Texture>) -> Self {
        Self { fuzz, albedo }
    }
}

impl Material for Metal {
    fn scatter(&self, _ray: &Ray, interaction: &Interaction) -> Option<(Spectrum, Ray)> {
        // (1) wo is the opposite of incoming ray (i.e. wo "goes away" from the intersection point),
        // so, wi = -wo.
        //
        // local_wo is expressed in a space where the up vector is 'z' and is also the normal vector to
        // the surface at the intersection point.
        //
        // So, computing the reflection of the wi vector (wi - 2*dot(wi, n)*n) where n is [0, 0, 1]
        // leads to [wix, wiy, -wiz]
        // with wi = -wo the end result is [-wox, -woy, woz]

        let Interaction { ref intersection, .. } = interaction;
        let Intersection { ref p, ref n, ref wo, .. } = intersection;

        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();

        let local_reflected = Vector3f::new(-local_wo.x, -local_wo.y, local_wo.z); // (1)
        let mut local_target = local_reflected + random_in_unit_sphere() * self.fuzz;
        local_target.normalize();

        if local_target.z > 0.0 {
            // <=> dot(local_target, n)
            let target = intersection.local_to_world(&local_target);
            let shift_avoid_acne = n * 0.001;
            let scattered_ray = Ray::new(&(p + &shift_avoid_acne), &target);
            let attenuation = self.albedo.shade(intersection);
            Some((attenuation, scattered_ray))
        }
        else {
            None
        }
    }
}
