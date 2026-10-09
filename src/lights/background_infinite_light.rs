use super::{Light, LightLiSample, LightType, VisibilityTester};
use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use crate::pdfs::sphere::SpherePdf;
use crate::pdfs::{DirectionDensity, Pdf};
use crate::samplers::Sampler;
use crate::spectrum::Spectrum;

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

    /// Draws a direction and reports the radiance arriving along it.
    ///
    /// Reference: PBR Book, 4ed, §12.5 — *Infinite Area Lights*.
    /// <https://pbr-book.org/4ed/Light_Sources/Infinite_Area_Lights>
    ///
    /// Same measure as [`UniformInfiniteLight`](super::UniformInfiniteLight) — a direction density
    /// of `1/4π`, no distance, no jacobian — and the same departure, plus one of its own: the
    /// gradient is not importance sampled, so the bright part of the sky is drawn no more often
    /// than the dark part. The value must agree with `le`, which answers the same question for a
    /// ray that escapes the scene.
    fn sample_li(&self, intersection: &Intersection, sampler: &mut dyn Sampler) -> Option<(LightLiSample, VisibilityTester)> {
        let sphere_pdf = SpherePdf {};

        let wi = sphere_pdf.generate(&sampler.get_2d());
        let pdf = sphere_pdf.value(&wi);
        let factor = 0.5 * (wi.y + 1.0);
        let spectrum = self.f * (1.0 - factor) + self.t * factor;

        let sample = LightLiSample {
            spectrum,
            wi,
            density: DirectionDensity::SolidAngle(pdf),
        };
        let tester = VisibilityTester::towards_infinity(&intersection.p, &wi);
        Some((sample, tester))
    }

    /// `1/4π` for every direction, from every point: the density of the [`SpherePdf`] that
    /// `sample_li` draws from, asked of that very object so the two cannot drift apart. The gradient
    /// plays no part, since it is not importance sampled.
    fn pdf_li(&self, _reference: &Vector3f, wi: &Vector3f) -> f64 {
        SpherePdf {}.value(wi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::colors;
    use crate::geom::vector2::Vector2u;
    use crate::samplers::IndependentSampler;

    /// A shaded point away from the origin, so that a `pdf_li` reading its reference by mistake
    /// would have something to read. Only `p` is read by `sample_li`.
    fn shaded() -> Intersection {
        Intersection {
            p: Vector3f::new(1.0, -2.0, 3.0),
            d: 1.0,
            n: Vector3f::new(0.0, 1.0, 0.0),
            wo: Vector3f::new(0.0, 1.0, 0.0),
            u: 0.0,
            v: 0.0,
            dpdu: Vector3f::new(1.0, 0.0, 0.0),
            dpdv: Vector3f::new(0.0, 0.0, 1.0),
        }
    }

    /// For every direction `sample_li` draws, `pdf_li` returns the density it came with — the
    /// contract multiple importance sampling relies on. Both read one `SpherePdf`, so they agree
    /// bit for bit.
    #[test]
    fn test_pdf_li_is_the_density_sample_li_draws_with() {
        let light = BackgroundInfiniteLight::new(colors::WHITE, Spectrum::new(0.5, 0.7, 1.0));
        let shaded = shaded();
        let mut sampler = IndependentSampler::new(0, &Vector2u::new(0, 0), 0);

        for _ in 0..1000 {
            let (sample, _) = light.sample_li(&shaded, &mut sampler).unwrap();
            let drawn_with = match sample.density {
                DirectionDensity::SolidAngle(pdf) => pdf,
                DirectionDensity::Dirac => panic!("a background infinite light draws with a density"),
            };

            assert_eq!(light.pdf_li(&shaded.p, &sample.wi), drawn_with, "wi = {}", sample.wi);
        }
    }
}
