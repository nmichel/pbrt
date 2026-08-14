use super::{Light, LightLiSample, LightType, VisibilityTester};
use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use crate::pdfs::sphere::SpherePdf;
use crate::pdfs::Pdf;
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

    fn sample_li(&self, intersection: &Intersection) -> Option<(LightLiSample, VisibilityTester)> {
        let sphere_pdf = SpherePdf {};

        let wi = sphere_pdf.generate();
        let pdf = sphere_pdf.value(&wi);
        let spectrum = self.i;

        let sample = LightLiSample { spectrum, wi, pdf };
        let tester = VisibilityTester::towards_infinity(&intersection.p, &wi);
        Some((sample, tester))
    }
}
