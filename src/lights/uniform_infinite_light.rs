use super::{Light, LightLiSample, LightType, VisibilityTester};
use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::pdfs::sphere::SpherePdf;
use crate::pdfs::Pdf;
use crate::samplers::Sampler;
use crate::spectrum::Spectrum;

pub struct UniformInfiniteLight {
    i: Spectrum,
}

impl UniformInfiniteLight {
    pub fn new(i: Spectrum) -> Self {
        UniformInfiniteLight { i }
    }
}

impl Light for UniformInfiniteLight {
    fn light_type(&self) -> LightType {
        LightType::Infinite
    }

    fn le(&self, _ray: &Ray) -> Spectrum {
        self.i.clone()
    }

    /// Draws a direction and reports the radiance arriving along it.
    ///
    /// Reference: PBR Book, 4ed, §12.5 — *Infinite Area Lights*.
    /// <https://pbr-book.org/4ed/Light_Sources/Infinite_Area_Lights>
    ///
    /// The radiance is the same everywhere and in every direction, so there is no distance and no
    /// change of measure: the density is that of the direction draw alone, `1/4π`.
    ///
    /// **A departure**: the draw covers the whole sphere, while only the hemisphere around the
    /// shaded normal can contribute — a bsdf answers black for the other half. The estimator stays
    /// unbiased, since the density covers the whole domain, but half of the shadow rays are spent
    /// on directions known in advance to be worth nothing.
    fn sample_li(&self, intersection: &Intersection, sampler: &mut dyn Sampler) -> Option<(LightLiSample, VisibilityTester)> {
        let sphere_pdf = SpherePdf {};

        let wi = sphere_pdf.generate(&sampler.get_2d());
        let pdf = sphere_pdf.value(&wi);
        let spectrum = self.i;

        let sample = LightLiSample { spectrum, wi, pdf };
        let tester = VisibilityTester::towards_infinity(&intersection.p, &wi);
        Some((sample, tester))
    }
}
