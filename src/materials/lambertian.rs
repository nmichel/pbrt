use super::{Material, ScatterDensity, ScatterInfo};
use crate::colors;
use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::shading_frame::{self, ShadingFrame};
use crate::geom::surface_point::SurfacePoint;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::pdfs::cosine::CosinePdf;
use crate::pdfs::Pdf;
use crate::samplers::Sampler;
use crate::spectrum::Spectrum;
use crate::textures::*;
use std::sync::Arc;

pub struct Lambertian {
    albedo: Arc<dyn Texture>,
}

impl Lambertian {
    pub fn new(albedo: Arc<dyn Texture>) -> Self {
        Self { albedo: albedo.clone() }
    }
}

impl Material for Lambertian {
    fn scatter(&self, _ray: &Ray, interaction: &Interaction, sampler: &mut dyn Sampler) -> Option<ScatterInfo> {
        let Interaction { ref intersection, .. } = interaction;
        let Intersection { ref p, ref n, ref wo, .. } = intersection;

        let frame = ShadingFrame::from(intersection);
        let local_wo = frame.world_to_local(wo);

        // The normal is the geometric one and is not turned towards the viewer, so a surface hit
        // from behind has `wo` below the hemisphere `CosinePdf` draws in. The reflected direction
        // belongs on the side of `wo`: drawing it on the other would send light *through* the
        // surface, where `f` and `pdf`, which both test `same_hemisphere`, say there is none.
        // Mirroring the draw keeps its density — `cos θ/π` depends on |cos θ| only.
        let pdf: CosinePdf = CosinePdf {};
        let mut local_wi: Vector3f = pdf.generate(&sampler.get_2d());
        if local_wo.z < 0.0 {
            local_wi.z = -local_wi.z;
        }
        let wi: Vector3f = frame.local_to_world(&local_wi);

        // The ray leaves on the side it was drawn on; stepping the other way would start it
        // behind the surface it is leaving, and its first hit would be that surface.
        let side = local_wi.z.signum();
        let shift_avoid_acne = n * (0.001 * side);
        let scattered_ray = Ray::new(&(p + &shift_avoid_acne), &wi);
        let attenuation = self.albedo.shade(&SurfacePoint::from(intersection)) / std::f64::consts::PI;
        let scattering_pdf = pdf.value(&local_wi);

        return Some(ScatterInfo::new(attenuation, scattered_ray, ScatterDensity::SolidAngle(scattering_pdf)));
    }

    fn f(&self, world_wo: &Vector3f, world_wi: &Vector3f, interaction: &Interaction) -> Spectrum {
        let Interaction { ref intersection, .. } = interaction;

        let frame = ShadingFrame::from(intersection);
        let wi = frame.world_to_local(world_wi);
        let wo = frame.world_to_local(world_wo);

        if shading_frame::same_hemisphere(&wo, &wi) {
            self.albedo.shade(&SurfacePoint::from(intersection)) / std::f64::consts::PI
        }
        else {
            colors::BLACK
        }
    }

    /// `cos θ / π` on the side of `wo`, 0 on the other — the derivation is on [`CosinePdf`].
    ///
    /// It asks the very [`CosinePdf`] that `scatter` draws from, so the two describe one
    /// distribution by construction rather than by agreement. The hemisphere test is the one `f`
    /// makes: where `f` is zero the density is too, and the two weights of multiple importance
    /// sampling vanish together.
    fn pdf(&self, world_wo: &Vector3f, world_wi: &Vector3f, interaction: &Interaction) -> f64 {
        let Interaction { ref intersection, .. } = interaction;

        let frame = ShadingFrame::from(intersection);
        let wi = frame.world_to_local(world_wi);
        let wo = frame.world_to_local(world_wo);

        if shading_frame::same_hemisphere(&wo, &wi) {
            CosinePdf {}.value(&wi)
        }
        else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::geom::vector2::Vector2u;
    use crate::geom::vector3;
    use crate::pdfs::sphere::SpherePdf;
    use crate::samplers::IndependentSampler;

    /// How closely `pdf` must reproduce the density `scatter` handed over. Both are `|cos θ|/π` of
    /// the same direction, but `pdf` recomputes cos θ from a world-space vector taken back into the
    /// shading frame, so they agree to rounding and not bit for bit.
    const DENSITY_TOLERANCE: f64 = 1e-12;

    const SAMPLES: usize = 100_000;

    /// The surface z = 0, normal `+z`.
    fn n() -> Vector3f {
        Vector3f::new(0.0, 0.0, 1.0)
    }

    fn lambertian() -> Arc<Lambertian> {
        Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::WHITE))))
    }

    /// That surface reached by a ray arriving from `wo`.
    fn seen_from(wo: &Vector3f) -> Interaction {
        Interaction {
            intersection: Intersection {
                p: Vector3f::zero(),
                d: 1.0,
                n: n(),
                wo: wo.normalized(),
                u: 0.5,
                v: 0.5,
                dpdu: Vector3f::new(1.0, 0.0, 0.0),
                dpdv: Vector3f::new(0.0, 1.0, 0.0),
            },
            material: lambertian(),
        }
    }

    /// Viewers on both faces: the normal is not turned towards the viewer, so the face behind it is
    /// a case `scatter` has to handle, not one it can assume away.
    fn viewers() -> [Vector3f; 4] {
        [
            Vector3f::new(0.0, 0.0, 1.0),
            Vector3f::new(0.6, -0.3, 0.2).normalized(),
            Vector3f::new(0.0, 0.0, -1.0),
            Vector3f::new(-0.5, 0.4, -0.3).normalized(),
        ]
    }

    /// For every direction `scatter` draws, `pdf` returns the density it came with — on the side of
    /// `wo`, whichever face that is.
    ///
    /// This is the contract multiple importance sampling relies on: the bsdf's density for a
    /// direction must not depend on who drew it. A `scatter` that drew on the wrong face would fail
    /// here twice — its direction would not be on the side of `wo`, and `pdf` would answer 0 for it.
    #[test]
    fn test_pdf_is_the_density_scatter_draws_with() {
        let material = lambertian();
        let mut sampler = IndependentSampler::new(0, &Vector2u::new(0, 0), 0);

        for wo in viewers() {
            let interaction = seen_from(&wo);
            let incoming = Ray::new(&wo, &-wo);

            for _ in 0..1000 {
                let sample = material.scatter(&incoming, &interaction, &mut sampler).unwrap();
                let wi = sample.scattered.direction;
                let drawn_with = match sample.density {
                    ScatterDensity::SolidAngle(pdf) => pdf,
                    ScatterDensity::Dirac => panic!("a lambertian draw has a density"),
                };

                let same_side = vector3::dot(&wi, &n()) * vector3::dot(&wo, &n()) > 0.0;
                assert!(same_side, "wo = {} drew wi = {} on the other face", wo, wi);

                let stated = material.pdf(&wo, &wi, &interaction);
                assert!(
                    (stated - drawn_with).abs() < DENSITY_TOLERANCE,
                    "wo = {}, wi = {}: scatter drew with {}, pdf states {}",
                    wo,
                    wi,
                    drawn_with,
                    stated
                );
            }
        }
    }

    /// `∫ pdf dω = 1` over the whole sphere of directions, for a viewer on either face.
    ///
    /// Estimated by drawing directions uniformly over the sphere, whose density is `1/4π`:
    /// `E[pdf(ω) / (1/4π)] = ∫ pdf dω`. This is the half of the contract the previous test cannot
    /// see. It only asks about directions `scatter` produces, so a `pdf` that answered `|cos θ|/π`
    /// on the *other* face too would pass it — and integrate to 2 here.
    #[test]
    fn test_pdf_integrates_to_one_for_a_viewer_on_either_face() {
        let material = lambertian();
        let uniform = SpherePdf {};
        let mut sampler = IndependentSampler::new(0, &Vector2u::new(0, 0), 0);

        for wo in viewers() {
            let interaction = seen_from(&wo);

            let mut total = 0.0;
            for _ in 0..SAMPLES {
                let w = uniform.generate(&sampler.get_2d());
                total += material.pdf(&wo, &w, &interaction) / uniform.value(&w);
            }
            let integral = total / SAMPLES as f64;

            assert!((integral - 1.0).abs() < 0.01, "wo = {}: ∫ pdf dω estimated at {}", wo, integral);
        }
    }
}
