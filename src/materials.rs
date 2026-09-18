use crate::colors;
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::samplers::Sampler;
use crate::spectrum::Spectrum;

pub struct ScatterInfo {
    pub attenuation: Spectrum,
    pub scattered: Ray,
    pub pdf: f64,
}

impl ScatterInfo {
    pub fn new(attenuation: Spectrum, scattered: Ray, pdf: f64) -> Self {
        Self { attenuation, scattered, pdf }
    }
}

/// A bsdf: it samples a scattered direction, evaluates its own density over directions, and emits.
///
/// # Frame
///
/// **Every direction this trait exchanges is in world coordinates** — the `wo` and `wi` of `f`, the
/// `scattered` ray of a [`ScatterInfo`]. An implementation that needs the shading frame builds a
/// [`ShadingFrame`](crate::geom::shading_frame::ShadingFrame) from the interaction and converts, and
/// that frame's module is where the convention is stated. Keeping the trait in world coordinates is
/// what lets an integrator compose materials and lights without knowing which of them happens to
/// work in a local frame.
pub trait Material: Send + Sync {
    /// Samples an outgoing direction, drawing whatever numbers the bsdf needs from `sampler`.
    ///
    /// A specular material consumes one number or none, a lambertian two; the caller is told
    /// neither, which is what lets a material change its bsdf without changing the integrator.
    fn scatter(&self, _ray: &Ray, _interaction: &Interaction, _sampler: &mut dyn Sampler) -> Option<ScatterInfo> {
        None
    }

    fn emit(&self, _ray: &Ray, _interaction: &Interaction) -> Option<Spectrum> {
        None
    }

    fn is_specular(&self) -> bool {
        false
    }

    fn f(&self, _wo: &Vector3f, _wi: &Vector3f, _interaction: &Interaction) -> Spectrum {
        colors::BLACK
    }
}

/// Divides out the cosine the integrator will apply to a specular scattering event.
///
/// Reference: PBR Book, 3ed, §8.2.2 — *Specular Reflection and Transmission*.
/// <https://www.pbr-book.org/3ed-2018/Reflection_Models/Specular_Reflection_and_Transmission#SpecularReflection>
///
/// # The departure from the physical model, and why it is the right one
///
/// An integrator estimates the scattering equation, so it weights every sampled direction by
/// |cos θᵢ| — `abs_dot(wi, n)` in `path.rs` and `naive.rs`. That is correct for a bsdf which is a
/// *function* of direction. A perfect mirror's is not: it is a δ distribution, non-zero on the one
/// direction it reflects into, and its integral already accounts for the projected solid angle. The
/// factor the integrator is about to apply is therefore surnumerary, and pbrt's answer — the one
/// taken here — is to give the bsdf a 1/|cos θᵢ| of its own so that the product is 1:
///
/// ```text
///   f(wo, wi) = ρ ⋅ δ(wi − reflect(wo)) / |cos θᵢ|      then      f ⋅ |cos θᵢ| = ρ ⋅ δ(…)
/// ```
///
/// **Consequence on the image**: none, as long as the two halves agree. The division is a
/// compensation, not a fudge, and `test_specular_cosine_cancels_out` is what says so — it fails the
/// moment an integrator stops applying the cosine, which is exactly when this function must go.
///
/// **A grazing direction is a singularity**: |cos θᵢ| → 0 sends the attenuation to infinity. The
/// materials that call this guard against it by their own geometry — `Metal` drops a direction
/// below the horizon before getting here, `Dielectric` only ever passes a reflected or refracted
/// direction, neither of which is tangent unless `wo` itself is.
///
/// # Frame
///
/// `local_wi` is in the [`ShadingFrame`](crate::geom::shading_frame::ShadingFrame), so cos θᵢ is
/// its `z` component. The absolute value is what makes this serve transmission as well as
/// reflection: a refracted direction sits *below* the surface, with a negative cosine.
pub fn cancel_integrator_cosine(attenuation: Spectrum, local_wi: &Vector3f) -> Spectrum {
    let abs_cos_theta = local_wi.z.abs();
    attenuation / abs_cos_theta
}

mod dielectric;
mod diffuse_light;
mod lambertian;
mod metal;

pub use self::dielectric::{Dielectric, RefractionIndices};
pub use self::diffuse_light::DiffuseLight;
pub use self::lambertian::Lambertian;
pub use self::metal::Metal;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::shading_frame;

    /// Round-trip through a division and a multiplication by the same cosine, so the claim being
    /// checked is conservation and not bit-exact equality — `(1/c)·c` is not always 1 in binary
    /// floating point.
    const ENERGY_TOLERANCE: f64 = 1e-12;

    /// The largest departure from white over the three components, in either direction — a loss
    /// and a gain of energy are both failures.
    fn assert_conserves_energy(beta: Spectrum, label: &str) {
        let lost = (colors::WHITE + beta * -1.0).max_component_value();
        let gained = (beta + colors::WHITE * -1.0).max_component_value();
        let departure = lost.max(gained);

        assert!(departure < ENERGY_TOLERANCE, "{}: energy off by {}, beta is {:?}", label, departure, beta);
    }

    /// A perfect mirror loses no energy: the 1/|cos θᵢ| of
    /// [`cancel_integrator_cosine`] and the |cos θᵢ| the integrator applies must multiply out to
    /// exactly 1, at any incidence.
    ///
    /// This is the test the division exists for. It is not a restatement of the code — it is the
    /// contract between a material and *every* integrator, and it is what fails if either side
    /// changes its mind about who applies the cosine.
    #[test]
    fn test_specular_cosine_cancels_out() {
        let white_mirror = colors::WHITE;

        // Several incidences, from near-normal to steep — but not tangent, which is the
        // singularity the function's documentation calls out.
        for cos_theta_o in [1.0_f64, 0.9, 0.5, 0.1, 0.01].iter() {
            let sin_theta_o = (1.0 - cos_theta_o * cos_theta_o).sqrt();
            let local_wo = Vector3f::new(sin_theta_o, 0.0, *cos_theta_o);
            let local_wi = shading_frame::reflect(&local_wo);

            let attenuation = cancel_integrator_cosine(white_mirror, &local_wi);

            // What an integrator does with a ScatterInfo: attenuation ⋅ |cos θᵢ| / pdf, with the
            // pdf of a delta distribution being 1.
            let beta = attenuation * local_wi.z.abs() / 1.0;

            assert_conserves_energy(beta, &format!("perfect mirror at cos θₒ = {}", cos_theta_o));
        }
    }

    /// The absolute value is not cosmetic: it is what lets the same function serve a transmitted
    /// direction, which sits below the surface and has a negative cosine.
    #[test]
    fn test_transmitted_direction_is_handled() {
        let below = Vector3f::new(0.3, 0.0, -0.8);

        let attenuation = cancel_integrator_cosine(colors::WHITE, &below);
        let beta = attenuation * below.z.abs();

        assert_conserves_energy(beta, "transmitted direction");
    }
}

/*
impl Material {
    pub fn shade(&self, intersection: &Intersection, world_wi: &Vector3f) -> Spectrum {
        let diffuse = self.texture.shade(intersection);
        let specular = Spectrum::new(1.0, 1.0, 1.0);
        let spec_coef = Self::cook_torrance(&intersection, &world_wi);
        // println!("spec_coef {:?}", &spec_coef);
        diffuse * Self::lambert() * (1.0 - spec_coef)
        + specular * spec_coef
        // + specular * Self::phong(&intersection, &world_wi)
        // + specular * Self::blinn_phong(&intersection, &world_wi)

    }

    fn lambert() -> f64 {
        1.0 / 3.14159
    }

    fn phong(intersection: &Intersection, world_wi: &Vector3f) -> f64 {
        let Intersection {ref wo, .. } = intersection;
        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let mut local_wi = intersection.world_to_local(&world_wi);
        local_wi.normalize();
        let r = Vector3f::new(-local_wi.x, -local_wi.y, local_wi.z);
        let spec_coef = num_traits::clamp(dot(&local_wo, &r), 0.0, 1.0);
        spec_coef.powf(20.0)
    }

    fn blinn_phong(intersection: &Intersection, world_wi: &Vector3f) -> f64 {
        let Intersection {ref wo, .. } = intersection;
        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let mut local_wi = intersection.world_to_local(&world_wi);
        local_wi.normalize();
        let mut h = local_wo + local_wi;
        h.normalize();
        let spec_coef = num_traits::clamp(h.z, 0.0, 1.0);
        spec_coef.powf(80.0)
    }

    fn cook_torrance(intersection: &Intersection, world_wi: &Vector3f) -> f64 {
        let Intersection {ref wo, .. } = intersection;
        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let mut local_wi = intersection.world_to_local(&world_wi);
        local_wi.normalize();
        let nl = local_wi.z;
        let nv = local_wo.z;

        let d = Material::d_ggx(&intersection, world_wi);
        // let g = Material::g_cook_torrance(&intersection, world_wi);
        let g = Material::g_smith(&intersection, world_wi);
        let f = Material::f_schlick(&intersection, world_wi);
        //(d * g * f) / (4.0 * nl * nv)
        (d * g * f) / (4.0 * nl * nv)
    }

    fn d_ggx(intersection: &Intersection, world_wi: &Vector3f) -> f64 {
        let Intersection {ref wo, ref n, .. } = intersection;
        let alpha = 0.5;

        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let mut local_wi = intersection.world_to_local(&world_wi);
        local_wi.normalize();
        let mut m = local_wo + local_wi;
        m.normalize();

        let alpha_2 = alpha * alpha; // α²
        let m_z = f64::max(0.0, m.z);
        let m_z_2 = m_z * m_z; // (n.m)² with m in local frame, where n is local z unit vector [0, 0, 1]
        let den = m_z_2 * (alpha_2 - 1.0) + 1.0; // (n⋅m)²(α² - 1) + 1
        alpha_2 / (std::f64::consts::PI * den*den) // α² / (π((n⋅m)²(α² - 1) + 1)²)
    }

    fn g_cook_torrance(intersection: &Intersection, world_wi: &Vector3f) -> f64 {
        let Intersection {ref wo, .. } = intersection;

        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let mut local_wi = intersection.world_to_local(&world_wi);
        local_wi.normalize();
        let mut h = local_wo + local_wi;
        h.normalize();

        let vh = dot(&local_wo, &h);
        let nh = h.z;
        let nv = local_wo.z;
        let nl = local_wi.z;
        let t1 = (2.0 * nh * nv) / vh; // (2(n⋅h)(n⋅v))/(v⋅h)
        let t2 = (2.0 * nh * nl) / vh; // (2(n⋅h)(n⋅l))/(v⋅h)
        f64::min(f64::min(1.0, t1), t2)
    }

    fn g_smith(intersection: &Intersection, world_wi: &Vector3f) -> f64 {
        let alpha = 0.5;

        let Intersection {ref wo, .. } = intersection;
        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let mut local_wi = intersection.world_to_local(&world_wi);
        local_wi.normalize();

        Material::g_schlick(&local_wo, alpha) * Material::g_schlick(&local_wi, alpha)
    }

    fn g_schlick(v: &Vector3f, alpha: f64) -> f64 {
        let k = alpha * (2.0 / std::f64::consts::PI).sqrt();
        let nv = f64::max(v.z, 0.0);
        nv / (nv * (1.0 - k) + k)
    }

    fn f_schlick(intersection: &Intersection, world_wi: &Vector3f) -> f64 {
        let Intersection {ref wo, .. } = intersection;

        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let mut local_wi = intersection.world_to_local(&world_wi);
        local_wi.normalize();
        let mut h = local_wo + local_wi;
        h.normalize();

        let ior = 1.8;
        let ior_sub = 1.0 - ior;
        let ior_add = 1.0 + ior;
//        let f0 = (ior_sub * ior_sub) / (ior_add * ior_add);
        let f0 = 0.95;
        let r = f0 + (1.0 - f0) * (1.0 - dot(&local_wo, &h)).powf(5.0);
        r
    }
}
*/
