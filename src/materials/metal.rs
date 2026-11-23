use super::{Material, ScatterInfo};
use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::shading_frame::{self, ShadingFrame};
use crate::geom::surface_point::SurfacePoint;
use crate::interaction::Interaction;
use crate::pdfs::sphere::SpherePdf;
use crate::pdfs::{DirectionDensity, Pdf};
use crate::samplers::Sampler;
use crate::textures::*;
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
    fn scatter(&self, _ray: &Ray, interaction: &Interaction, sampler: &mut dyn Sampler) -> Option<ScatterInfo> {
        let Interaction { ref intersection, .. } = interaction;
        let Intersection { ref p, ref n, ref wo, .. } = intersection;

        let frame = ShadingFrame::from(intersection);
        let mut local_wo = frame.world_to_local(&wo);
        local_wo.normalize();

        let local_reflected = shading_frame::reflect(&local_wo);

        // `fuzz` blurs the mirror direction by displacing it before renormalising. The offset is
        // drawn on the *surface* of the sphere of radius `fuzz`, so its magnitude is exactly
        // `fuzz`, and `SpherePdf` — which already derives the uniform sphere map — is where that
        // draw comes from.
        //
        // **A departure worth naming.** This blur is not a physical model of a rough conductor: it
        // is the phenomenological `fuzz` of "Ray Tracing in One Weekend", and it has no microfacet
        // distribution, no shadowing-masking term and no Fresnel behind it. Sampling the sphere
        // rather than the ball it bounds is therefore a change *within* an approximation, not a
        // change to a physical quantity — but it is not invisible: a ball-uniform offset has mean
        // magnitude ¾·fuzz, so the perturbation here is about a third larger. At large `fuzz` more
        // targets fall below the horizon and are absorbed by the `local_target.z > 0.0` test
        // below, so a fuzzy metal is slightly darker. Replacing the blur with a microfacet BRDF is
        // the fix that would make the question moot.
        let fuzz_offset = SpherePdf {}.generate(&sampler.get_2d()) * self.fuzz;
        let mut local_target = local_reflected + fuzz_offset;
        local_target.normalize();

        // Above the horizon: in the shading frame the z component *is* cos θ, so this is the test
        // `dot(local_target, n) > 0` without the dot product. A blurred direction that fell below
        // the surface is absorbed rather than reflected into the geometry.
        if local_target.z > 0.0 {
            let target = frame.local_to_world(&local_target);
            let shift_avoid_acne = n * 0.001;
            let scattered_ray = Ray::new(&(p + &shift_avoid_acne), &target);

            // A δ sample carries the reflectance ρ itself, the fraction of energy sent along the one
            // direction: the cosine and the δ have cancelled out of `f · |cos θᵢ| / p` already, which
            // is what `DirectionDensity::Dirac` tells the integrator.
            let attenuation = self.albedo.shade(&SurfacePoint::from(intersection));

            // **Not a δ as soon as `fuzz > 0`**, and declared one anyway. The blurred direction has a
            // density, but nobody has derived it — the offset is uniform on a sphere of radius
            // `fuzz`, and its image after renormalisation is not a distribution with a closed form.
            // Without a density there is nothing to divide by, so the blur is treated as part of the
            // mirror: unbiased with respect to its own model, which is already not a physical one.
            Some(ScatterInfo::new(attenuation, scattered_ray, DirectionDensity::Dirac))
        }
        else {
            None
        }
    }

    fn is_specular(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::colors;
    use crate::geom::vector2::Vector2u;
    use crate::geom::vector3::{self, Vector3f};
    use crate::samplers::IndependentSampler;

    /// How far the reflected direction may sit from the mirror one: both come out of a change of
    /// frame and back, so they agree to rounding and not bit for bit.
    const DIRECTION_TOLERANCE: f64 = 1e-12;

    /// A white mirror, which is the case where the energy claim has an exact value: everything that
    /// arrives leaves.
    fn white_mirror() -> Arc<Metal> {
        Arc::new(Metal::new(0.0, Arc::new(PlainColor::new(colors::WHITE))))
    }

    /// The surface z = 0 facing `+z`, reached by a ray arriving from `wo`.
    fn seen_from(wo: Vector3f, material: Arc<Metal>) -> Interaction {
        Interaction {
            intersection: Intersection {
                p: Vector3f::zero(),
                d: 1.0,
                n: Vector3f::new(0.0, 0.0, 1.0),
                wo: wo.normalized(),
                u: 0.5,
                v: 0.5,
                dpdu: Vector3f::new(1.0, 0.0, 0.0),
                dpdv: Vector3f::new(0.0, 1.0, 0.0),
            },
            material,
            area_light: None,
        }
    }

    /// A perfect white mirror loses no energy, at any incidence, grazing ones included.
    ///
    /// An integrator does `β ← β · attenuation` with a [`DirectionDensity::Dirac`] sample, so the
    /// attenuation *is* the fraction of energy carried on, and for a white mirror it must be exactly
    /// 1 — not 1 to a tolerance: the reflectance is handed over untouched, with no cosine divided
    /// out of it that the integrator would have to multiply back in. The incidences go down to
    /// cos θₒ = 0.001, where such a division would have been the dominant term.
    ///
    /// The direction is checked against the world-space formula `r = −wo + 2 (wo ⋅ n) n`, which
    /// does not go through the shading frame the material uses.
    #[test]
    fn test_a_white_mirror_loses_no_energy() {
        let n = Vector3f::new(0.0, 0.0, 1.0);
        let mut sampler = IndependentSampler::new(0, &Vector2u::new(0, 0), 0);

        for cos_theta_o in [1.0_f64, 0.9, 0.5, 0.1, 0.01, 0.001] {
            let sin_theta_o = (1.0 - cos_theta_o * cos_theta_o).sqrt();
            let wo = Vector3f::new(sin_theta_o, 0.0, cos_theta_o);
            let interaction = seen_from(wo, white_mirror());
            let incoming = Ray::new(&wo, &-wo);

            let sample = white_mirror().scatter(&incoming, &interaction, &mut sampler).unwrap();

            assert!(matches!(sample.density, DirectionDensity::Dirac), "cos θₒ = {}", cos_theta_o);
            assert_eq!(sample.attenuation, colors::WHITE, "energy lost or gained at cos θₒ = {}", cos_theta_o);

            let mirror = -wo + n * (2.0 * vector3::dot(&wo, &n));
            let departure = (sample.scattered.direction - mirror).length();
            assert!(
                departure < DIRECTION_TOLERANCE,
                "reflected {} off the mirror direction at cos θₒ = {}",
                departure,
                cos_theta_o
            );
        }
    }
}
