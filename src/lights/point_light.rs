use super::{Light, LightLiSample, LightType, VisibilityTester};
use crate::geom::intersectable::Intersection;
use crate::geom::transform::Transform;
use crate::geom::vector3::Vector3f;
use crate::samplers::Sampler;
use crate::spectrum::Spectrum;

pub struct PointLight {
    t: Box<Transform>,
    i: Spectrum,
}

impl PointLight {
    pub fn new(t: Box<Transform>, i: Spectrum) -> Self {
        PointLight { t, i }
    }
}

impl Light for PointLight {
    fn light_type(&self) -> LightType {
        LightType::Point
    }

    /// Reports what reaches `intersection` from the single point this light occupies.
    ///
    /// Reference: PBR Book, 4ed, §12.2 — *Point Lights*.
    /// <https://pbr-book.org/4ed/Light_Sources/Point_Lights>
    ///
    /// # Why there is nothing to estimate
    ///
    /// A point has no area, so it has no radiance — radiance is a flux per unit *projected area*.
    /// What it has is an intensity `I`, in W·sr⁻¹, and the irradiance it delivers is
    ///
    /// ```text
    /// [1]  E = I · cos θ / d²
    /// ```
    ///
    /// The `1/d²` is the solid angle the *receiver* subtends from the source, which is why it sits
    /// in the returned radiance here and in the returned density for a source with an area. All the
    /// light arrives from one direction, so the integral collapses onto it: there is no estimate,
    /// no variance, and the `sampler` is untouched.
    ///
    /// `pdf` is `1.0` so that the integrator's division leaves [1] alone — see [`LightLiSample`],
    /// and `docs/sources_de_lumiere.md` §5 for what that costs multiple importance sampling.
    fn sample_li(&self, intersection: &Intersection, _sampler: &mut dyn Sampler) -> Option<(LightLiSample, VisibilityTester)> {
        let world_light_pos = self.t.transform_point_to_world(&Vector3f::new(0.0, 0.0, 0.0));
        let wi = &world_light_pos - &intersection.p;
        let spectrum = &self.i / wi.squared_length();

        let sample = LightLiSample {
            spectrum: spectrum,
            wi: wi.normalized(),
            pdf: 1.0,
        };
        let tester = VisibilityTester::between(&intersection.p, &world_light_pos);
        Some((sample, tester))
    }
}
