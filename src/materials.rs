use crate::colors;
use crate::geom::ray::Ray;
use crate::geom::surface_point::SurfacePoint;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::pdfs::DirectionDensity;
use crate::samplers::Sampler;
use crate::spectrum::Spectrum;
use std::sync::Arc;

pub struct ScatterInfo {
    pub attenuation: Spectrum,
    pub scattered: Ray,
    pub density: DirectionDensity,
}

impl ScatterInfo {
    pub fn new(attenuation: Spectrum, scattered: Ray, density: DirectionDensity) -> Self {
        Self {
            attenuation,
            scattered,
            density,
        }
    }
}

/// A surface that emits light of its own, evaluated at a point on it.
///
/// Reference: PBR Book, 4ed, §12.4 — *Light Sources*, « Area Lights ».
/// <https://pbr-book.org/4ed/Light_Sources/Area_Lights>
///
/// # Frame
///
/// `sp.n` and `w` are both in world coordinates, and `w` points **away** from the surface, along
/// the direction the radiance is leaving in. So `sp.n ⋅ w` is the cosine of the angle between the
/// normal and that direction, and its sign says which face is being asked about.
///
/// That sign is the whole of the convention, and it is what a caller gets wrong. An eye path
/// already holds the direction it wants — `Intersection::wo` looks back along the ray that
/// arrived. Something sampling a source towards a shaded point holds the opposite, a direction
/// pointing *at* the surface, and has to negate it first. Handed the un-negated one, a one-sided
/// emitter answers for the face opposite the one being asked about: the face it leaves dark.
///
/// # Why a trait of its own, rather than one more method on `Material`
///
/// A [`SurfacePoint`] and a direction are everything an emitted radiance depends on. `Material`
/// cannot say that: its methods take an `Interaction`, which carries the ray that found the
/// surface, and a point *drawn* on a surface — uniformly over its area, to estimate an integral
/// over that surface — has no ray behind it. A caller holding such a point could not build one.
///
/// So this is the face of a material that a consumer other than the eye can use, and
/// [`Material::emitter`] is how that consumer reaches it.
///
/// # Zero is a radiance
///
/// The return type is a `Spectrum` and not an `Option`: a surface that emits nothing towards `w` —
/// because `w` is on the face it does not light — emits *zero*, which is a value. Whether a
/// material emits **at all** is the separate question [`Material::emitter`] answers, and keeping
/// the two apart is what stops one `None` from meaning both.
pub trait Emitter: Send + Sync {
    /// The radiance leaving `sp` towards `w`.
    fn l(&self, sp: &SurfacePoint, w: &Vector3f) -> Spectrum;
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

    /// The radiance this surface emits towards the viewer, or `None` if it is not an emitter.
    ///
    /// `None` is the answer of a material that does not emit — the default below, which is what
    /// every bsdf in the project keeps. It is **not** how an emitter reports a dark face: that is
    /// `Some(BLACK)`, zero radiance being a value. An integrator therefore never has to know which
    /// of the two it is looking at.
    fn emit(&self, _ray: &Ray, _interaction: &Interaction) -> Option<Spectrum> {
        None
    }

    /// The emitting face of this material, when it has one.
    ///
    /// Rust has no trait upcast, and whoever needs to know holds an `Arc<dyn Material>` by the time
    /// the question arises — the concrete type is gone exactly where it would have answered. Asking
    /// the material keeps "do I emit" a property it states, rather than one a caller infers from a
    /// type it no longer has.
    ///
    /// The receiver is owned so that the answer is a handle the caller keeps, and it is consumed:
    /// a caller that also needs the material clones the `Arc` first. That cost is paid once per
    /// object, when a scene is built — [`Self::emit`] is the one on the shading path.
    fn emitter(self: Arc<Self>) -> Option<Arc<dyn Emitter>> {
        None
    }

    fn f(&self, _wo: &Vector3f, _wi: &Vector3f, _interaction: &Interaction) -> Spectrum {
        colors::BLACK
    }

    /// The density, per unit solid angle, with which [`Self::scatter`] would have drawn `wi`
    /// leaving towards `wo`.
    ///
    /// Reference: PBR Book, 4ed, §9.1.2 — *BSDF Representation*, `BxDF::PDF`.
    /// <https://pbr-book.org/4ed/Reflection_Models/BSDF_Representation>
    ///
    /// # What asks for it
    ///
    /// `scatter` already hands over the density of the direction it draws. This answers for a
    /// direction drawn by *someone else* — a point sampled on a light, by next event estimation —
    /// which multiple importance sampling has to weigh against the bsdf's own chance of having
    /// found it. The two must describe the same distribution: for every `wi` that `scatter` can
    /// produce, this returns the density it came with.
    ///
    /// # Frame
    ///
    /// The trait's: `wo` and `wi` in world coordinates, both pointing away from the surface, `wo`
    /// towards the viewer as [`Intersection::wo`](crate::geom::intersectable::Intersection) does.
    ///
    /// # Why 0 is the default, and the rule it imposes
    ///
    /// Zero is the right answer for a material whose `f` is zero everywhere, which is what the
    /// default `f` above says: a δ, whose draw comes as [`DirectionDensity::Dirac`] and has no
    /// density at any direction, or a material that scatters nothing. Next event estimation gets
    /// nothing from such a material whatever its weight, so the zero is harmless.
    ///
    /// **It is wrong for anything else.** A material that defines `f` and keeps this default
    /// would draw with one density in `scatter` and declare zero here; the two weights of multiple
    /// importance sampling would then no longer add up to 1, and the image would be biased. So:
    /// **a material that overrides `f` overrides `pdf`.**
    fn pdf(&self, _wo: &Vector3f, _wi: &Vector3f, _interaction: &Interaction) -> f64 {
        0.0
    }
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
    use crate::textures::PlainColor;

    /// The trait's default, asked the way a scene builder asks it: through an `Arc<dyn Material>`,
    /// where the concrete type is gone.
    ///
    /// Asking it that way is also what checks that an `Arc<Self>` receiver leaves `Material`
    /// usable as a trait object — a call on a bare `Lambertian` would compile whether or not it
    /// did. A material that does not emit has no emitting face, and that is the half of the
    /// answer a load-time diagnostic will read.
    #[test]
    fn test_a_material_that_does_not_emit_has_no_emitter() {
        let material: Arc<dyn Material> = Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::WHITE))));

        assert!(material.emitter().is_none());
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
