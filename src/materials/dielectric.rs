use super::{Material, ScatterInfo};
use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::textures::*;
use crate::utils::random_double;
use std::sync::Arc;

#[non_exhaustive]
pub struct RefractionIndices;

impl RefractionIndices {
    pub const DIAMOND: f64 = 2.417;
    pub const GLASS: f64 = 1.517;
    pub const WATER: f64 = 1.333;
}

pub struct Dielectric {
    ref_idx: f64,
    albedo: Arc<dyn Texture>,
}

impl Dielectric {
    pub fn new(ref_idx: f64, albedo: Arc<dyn Texture>) -> Self {
        Self { ref_idx, albedo }
    }
}

impl Material for Dielectric {
    fn scatter(&self, _ray: &Ray, interaction: &Interaction) -> Option<ScatterInfo> {
        // see https://graphics.stanford.edu/courses/cs148-10-summer/docs/2006--degreve--reflection_refraction.pdf

        let Interaction { ref intersection, .. } = interaction;
        let Intersection { ref p, ref n, ref wo, .. } = intersection;

        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();

        // Always compute the reflected vector (in local frame), as it is likely to be used (though not
        // certain)
        let local_reflected = Vector3f::new(-local_wo.x, -local_wo.y, local_wo.z);

        let mut attenuation = self.albedo.shade(intersection);
        let local_outward_normal: Vector3f;
        let world_outward_normal: Vector3f;
        let ni: f64;
        let nt: f64;

        // The cosine of incident ray and the local normal tells us
        // if we are entering of leaving the volume.
        //
        // In local frame where n = z = (0, 0, 1), pointing outward the volume,
        // this cosine is the z coord of the incident ray.
        //
        if local_wo.z <= 0.0 {
            // Ray's leaving volume

            // Use reverted normal such as it lies in the same half-space as the incident ray.
            local_outward_normal = Vector3f::new(0.0, 0.0, -1.0);
            world_outward_normal = n * -1.0;

            // We are leaving the volume. Adjust ni and nt.
            ni = self.ref_idx;
            nt = 1.0;
        }
        else {
            // Ray's entering volume

            // Use the local normal (as it already pertains to the spame half-space thant
            // the incident ray)
            local_outward_normal = Vector3f::new(0.0, 0.0, 1.0);
            world_outward_normal = *n;

            // We are entering the volume. Adjust ni and nt.
            ni = 1.0;
            nt = self.ref_idx;
        }

        let ni_over_nt = ni / nt;

        match refract(&local_wo, &local_outward_normal, ni_over_nt) {
            Some(local_refracted) => {
                let reflectance = fresnel(local_wo, local_outward_normal, ni, nt);
                let local_scatter_direction: Vector3f;
                let scattered_ray_origin: Vector3f;
                if random_double() < reflectance {
                    local_scatter_direction = local_reflected;
                    let shift_avoid_acne = world_outward_normal * 0.001;
                    scattered_ray_origin = p + &shift_avoid_acne;
                }
                else {
                    local_scatter_direction = local_refracted;
                    let shift_avoid_acne = world_outward_normal * -0.001;
                    scattered_ray_origin = p + &shift_avoid_acne;
                };
                let scatter_direction = intersection.local_to_world(&local_scatter_direction);
                let scattered_ray = Ray::new(&scattered_ray_origin, &scatter_direction);

                // see https://www.pbr-book.org/3ed-2018/Reflection_Models/Specular_Reflection_and_Transmission#SpecularReflection
                // Handling of extra cosine because of delta distribution
                let abs_cos_theta = local_scatter_direction.z.abs();
                attenuation = attenuation / abs_cos_theta;

                Some(ScatterInfo::new(attenuation, scattered_ray, 1.0))
            }
            None => {
                // Total reflection
                let target = intersection.local_to_world(&local_reflected);
                let shift_avoid_acne = world_outward_normal * 0.001;
                let scattered_ray = Ray::new(&(p + &shift_avoid_acne), &target);

                // see https://www.pbr-book.org/3ed-2018/Reflection_Models/Specular_Reflection_and_Transmission#SpecularReflection
                // Handling of extra cosine because of delta distribution
                let abs_cos_theta = local_reflected.z.abs();
                attenuation = attenuation / abs_cos_theta;

                Some(ScatterInfo::new(attenuation, scattered_ray, 1.0))
            }
        }
    }

    fn is_specular(&self) -> bool {
        true
    }
}

fn refract(wo: &Vector3f, n: &Vector3f, ni_over_nt: f64) -> Option<Vector3f> {
    // sin² ϴₜ = (ηᵢ / ηₜ)² sin² ϴᵢ
    // sin² ϴᵢ = 1 - cos² ϴᵢ
    // cos² ϴᵢ = i.n
    let cos_theta_i = vector3::dot(&wo, n); // in local frame <=> wo.z * n.z
    let sin2_theta_i = 1.0 - cos_theta_i * cos_theta_i;
    let sin2_theta_t = ni_over_nt * ni_over_nt * sin2_theta_i;
    let discriminant = 1.0 - sin2_theta_t;
    if discriminant > 0.0 {
        // sin² ϴₜ + cos² ϴₜ = 1
        // cos² ϴₜ = 1 - sin² ϴₜ = discrimant
        // cos ϴₜ = √discrimant
        let cos_theta_t = discriminant.sqrt();

        // Refraction formula needs incident vector pointing at the intersection point
        // whereas wo points the opposite direction.
        let wi = wo * -1.0;

        // t = (ηᵢ / ηₜ)i + [(ηᵢ / ηₜ) cos ϴᵢ - √(1 - sin² ϴₜ)]
        // <=> t = (ηᵢ / ηₜ)i + [(ηᵢ / ηₜ) cos ϴᵢ - cos ϴₜ]
        let mut t = wi * ni_over_nt + n * (ni_over_nt * cos_theta_i - cos_theta_t);
        t.normalize();
        Some(t)
    }
    else {
        // Total internal reflection
        None
    }
}

/// Compute reflectance using refectance equations.
///
/// # Arguments
///
/// * `wo` - Opposite of incident ray direction (i.e. points outward the intersection point)
/// * `normal` - Normal vector to the interface, pointing in the same direction as `wo`
/// * `n1`- Refractive index of the material the incident ray comes from
/// * `n2`- Refractive index of the material the refracted ray goes into
///
fn fresnel(wo: Vector3f, normal: Vector3f, n1: f64, n2: f64) -> f64 {
    let n1_over_n2 = n1 / n2;
    let cos_theta_i = wo.z * normal.z;
    let cos_theta_t = (1.0 - n1_over_n2 * n1_over_n2 * (1.0 - cos_theta_i * cos_theta_i)).sqrt();

    // (27a)
    let r_ortho_root = (n1 * cos_theta_i - n2 * cos_theta_t) / (n1 * cos_theta_i + n2 * cos_theta_t);
    let r_ortho = r_ortho_root * r_ortho_root;

    // (27b)
    let r_parallel_root = (n2 * cos_theta_i - n1 * cos_theta_t) / (n2 * cos_theta_i + n1 * cos_theta_t);
    let r_parallel = r_parallel_root * r_parallel_root;

    (r_ortho + r_parallel) * 0.5 // (29a)
}

/// Use Schlick's approximation to compute reflectance.
///
/// # Arguments
///
/// * `wo` - Opposite of incident ray direction (i.e. points outward the intersection point)
/// * `normal` - Normal vector to the interface, pointing in the same direction as `wo`
/// * `n1`- Refractive index of the material the incident ray comes from
/// * `n2`- Refractive index of the material the refracted ray goes into
///
fn _schlick(wo: Vector3f, normal: Vector3f, n1: f64, n2: f64) -> f64 {
    let r0 = (n1 - n2) / (n1 + n2);
    let r0_2 = r0 * r0;
    let cos_theta_i = wo.z * normal.z;
    let cosine = if n1 <= n2 {
        // cosine = cos ϴᵢ    (32)
        cos_theta_i
    }
    else {
        // cosine = cos ϴₜ    (32)
        //
        // cos² ϴₜ + sin² ϴₜ = 1
        // cos² ϴₜ = 1 - sin² ϴₜ
        // cos² ϴₜ = 1 - (ηᵢ / ηₜ)²(1 - cos² ϴᵢ)     (23)
        // cos ϴₜ = √(1 - (ηᵢ / ηₜ)²(1 - cos² ϴᵢ))
        let n1_over_n2 = n1 / n2;
        (1.0 - n1_over_n2 * n1_over_n2 * (1.0 - cos_theta_i * cos_theta_i)).sqrt()
    };
    return r0_2 + (1.0 - r0_2) * (1.0 - cosine).powf(5.0);
}
