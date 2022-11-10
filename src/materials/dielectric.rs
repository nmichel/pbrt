use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::spectrum::Spectrum;
use crate::textures::*;
use crate::utils::random_double;
use std::sync::Arc;
use super::Material;

pub struct Dielectric {
    ref_idx: f64,
    albedo: Arc<dyn Texture>
}

impl Dielectric {
    pub fn new(ref_idx: f64, albedo: Arc<dyn Texture>) -> Self {
        Self { ref_idx, albedo }
    }
}

impl Material for Dielectric {
    fn scatter(&self, _ray: &Ray, interaction: &Interaction) -> Option<(Spectrum, Ray)> {
        let Interaction { ref intersection, .. } = interaction;
        let Intersection { ref p, n: _, ref wo, .. } = intersection;

        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let local_reflected = Vector3f::new(-local_wo.x, -local_wo.y, local_wo.z);

        let attenuation = self.albedo.shade(intersection);       
        let local_outward_normal: Vector3f;
        let ni_over_nt: f64;

        let mut cosine: f64;

        if local_wo.z <= 0.0 {
            // Ray's leaving volume
            local_outward_normal = Vector3f::new(0.0, 0.0, -1.0);
            ni_over_nt = self.ref_idx;
            cosine = -local_wo.z;
            cosine = (1.0 - self.ref_idx * self.ref_idx * (1.0 - cosine*cosine)).sqrt();
        }
        else {
            // Ray's entering volume
            local_outward_normal = Vector3f::new(0.0, 0.0, 1.0);
            ni_over_nt = 1.0 / self.ref_idx;
            cosine = local_wo.z;
        }
        match refract(&local_wo, &local_outward_normal, ni_over_nt) {
            Some(local_refracted) => {
                let reflect_prob = schlick(cosine, self.ref_idx);
                let local_scatter_direction = if random_double() < reflect_prob {
                    local_reflected
                }
                else {
                    local_refracted
                };
                let scatter_direction = intersection.local_to_world(&local_scatter_direction);
                let scattered_ray = Ray::new(&(p + &(&scatter_direction * 0.001)), &scatter_direction);
                Some((attenuation, scattered_ray))
            },
            None => {
                // Total reflection
                let target = intersection.local_to_world(&local_reflected);
                let scattered_ray = Ray::new(p, &target);
                Some((attenuation, scattered_ray))
            }
        }
    }
}

fn refract(wi: &Vector3f, n: &Vector3f, ni_over_nt: f64) -> Option<Vector3f> {
    let cos_theta_i = vector3::dot(&wi, n);
    let sin2_theta_i = 1.0 - cos_theta_i * cos_theta_i;
    let sin2_theta_t = ni_over_nt * ni_over_nt * sin2_theta_i;
    let discriminant = 1.0 - sin2_theta_t;
    if discriminant > 0.0 {
        let cos_theta_t = discriminant.sqrt();
        let mut t = wi * -ni_over_nt + n * (ni_over_nt * cos_theta_i - cos_theta_t);
        t.normalize();
        Some(t)
    }
    else {
        None
    }
}

fn schlick(cosine: f64, ref_idx: f64) -> f64 {
    let r = (1.0 - ref_idx) / (1.0 + ref_idx);
    let r2 = r * r;
    return r2 + (1.0 - r2)*(1.0 - cosine).powf(5.0);
}
