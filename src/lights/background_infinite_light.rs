use super::{Light, LightLiSample, LightType, VisibilityTester};
use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector2::Vector2f;
use crate::pdfs::sphere::SpherePdf;
use crate::pdfs::Pdf;
use crate::spectrum::Spectrum;
use crate::utils::random_double;

pub struct BackgroundInfiniteLight {
    f: Spectrum,
    t: Spectrum,
}

impl BackgroundInfiniteLight {
    pub fn new(f: Spectrum, t: Spectrum) -> Self {
        BackgroundInfiniteLight { f, t }
    }
}

impl Light for BackgroundInfiniteLight {
    fn light_type(&self) -> LightType {
        LightType::Infinite
    }

    fn le(&self, ray: &Ray) -> Spectrum {
        let unit_direction = ray.direction.normalized();
        let factor = 0.5 * (unit_direction.y + 1.0);
        self.f * (1.0 - factor) + self.t * factor
    }

    fn sample_li(&self, intersection: &Intersection) -> Option<(LightLiSample, VisibilityTester)> {
        let sphere_pdf = SpherePdf {};

        let u = Vector2f::new(random_double(), random_double());
        let wi = sphere_pdf.generate(&u);
        let pdf = sphere_pdf.value(&wi);
        let factor = 0.5 * (wi.y + 1.0);
        let spectrum = self.f * (1.0 - factor) + self.t * factor;

        let sample = LightLiSample { spectrum, wi, pdf };
        let tester = VisibilityTester::towards_infinity(&intersection.p, &wi);
        Some((sample, tester))
    }
}
