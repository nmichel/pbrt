//! A surface that lights a scene: an emitter spread over an area, sampled by drawing points of it.
//!
//! Reference: PBR Book, 4ed, §12.4 — *Light Sources*, « Area Lights », and §4.2.3 — « Integrals
//! over Area », whose equation (4.9) is the change of measure below.
//! <https://pbr-book.org/4ed/Light_Sources/Area_Lights>
//! <https://pbr-book.org/4ed/Radiometry,_Spectra,_and_Color/Working_with_Radiometric_Integrals#IntegralsoverArea>
//!
//! # Two participants, one surface
//!
//! An area light is a shape one can draw points of, plus something that says what those points
//! emit. It holds neither the object that is *seen* nor a geometry of its own: the shape it samples
//! is the very [`Arc`] the visible object renders, already placed in world space, so the two cannot
//! drift apart. Nothing here has to agree with anything — there is only one geometry.
//!
//! # The change of measure, which is the whole of this file
//!
//! An integrator works in **solid angle**: it divides a contribution by `pdf` where the sample is a
//! *direction*. A shape draws in **area**. The bridge between the two densities is the jacobian of
//! the map from one to the other, and getting it wrong is invisible to the eye — it scales an image
//! by a constant.
//!
//! Write `p` for the shaded point, `pₗ` for the point drawn on the source, `nₗ` for the normal
//! there, `d = ‖pₗ − p‖`, and `θₗ` for the angle at the source between `nₗ` and the direction back
//! towards `p`. A patch `dA` of the source, seen from `p`, subtends
//!
//! ```text
//! [1]  dω = dA · cos θₗ / d²
//! ```
//!
//! — `cos θₗ` because the patch is foreshortened by its tilt away from `p`, and `1/d²` because a
//! fixed patch subtends less angle the further it sits. Densities transform by the reciprocal of
//! that jacobian, since the same event must carry the same probability under either measure:
//!
//! ```text
//! [2]  p(ω) = p(A) · dA/dω = p(A) · d² / cos θₗ
//! ```
//!
//! **The inverse-square law is [1], and nothing else.** It is already in the density, so a
//! `sample_li` that also divided its radiance by `d²` would darken the image by `d²` a second time.
//! The radiance this returns is therefore the radiance the source emits, undiminished: radiance
//! along a ray does not fall off with distance, and what falls off is the solid angle the source
//! occupies — which is exactly what [2] accounts for.
//!
//! **`cos θₗ` is not under an absolute value**, where the general jacobian would put one. Emission
//! here is one-sided ([`Emitter`]), so a shaded point on the unlit face receives nothing at all,
//! and that case is answered with "no sample" before the division is reached. The absolute value
//! would matter for a two-sided emitter; this does not have one.
//!
//! **`cos θₗ → 0` makes the density explode**, and that is correct rather than a defect: a source
//! seen edge-on subtends almost no solid angle, so a point drawn on it stands for a vanishing
//! fraction of the directions leaving `p`, and its contribution is divided by a large number. The
//! variance is real, and it is what sampling by solid angle would remove.
//!
//! # A departure: the draw is uniform over the area
//!
//! Points are drawn uniformly over the surface, not over the solid angle it subtends from the
//! shaded point. The estimator stays **unbiased** — [2] is exact — but it is noisy when the source
//! is large and seen at a grazing angle, because most of the points drawn then land where they
//! matter least. pbrt samples the solid angle directly from a reference point; that is the next
//! step, and it costs variance rather than correctness to postpone.

use std::sync::Arc;

use super::{Light, LightLiSample, LightType, VisibilityTester};
use crate::geom::intersectable::Intersection;
use crate::geom::vector3;
use crate::materials::Emitter;
use crate::samplers::Sampler;
use crate::shapes::AreaSampleable;

pub struct AreaLight {
    shape: Arc<dyn AreaSampleable>,
    emitter: Arc<dyn Emitter>,
}

impl AreaLight {
    pub fn new(shape: Arc<dyn AreaSampleable>, emitter: Arc<dyn Emitter>) -> Self {
        Self { shape, emitter }
    }
}

impl Light for AreaLight {
    fn light_type(&self) -> LightType {
        LightType::Area
    }

    /// Draws a point of the source and reports what reaches `intersection` from it.
    ///
    /// The steps are those of the module header: draw in area measure, build the direction, convert
    /// the density to solid angle by [2], and read the radiance at the very point drawn.
    ///
    /// # The three ways there is no sample
    ///
    /// All three answer `None` rather than a sample carrying zero, because a sample worth nothing
    /// still costs a shadow ray.
    ///
    /// - **The drawn point *is* the shaded point.** `d² = 0` leaves no direction to speak of.
    /// - **The shaded point lies on the unlit face**, `cos θₗ ≤ 0`. The source emits nothing that
    ///   way, and this is also what keeps [2] from dividing by zero.
    /// - **The density is not a usable number.** `d²/cos θₗ` overflows for a point close to the
    ///   plane of the source, and a density of zero or infinity cannot be divided by.
    fn sample_li(&self, intersection: &Intersection, sampler: &mut dyn Sampler) -> Option<(LightLiSample, VisibilityTester)> {
        let sample = self.shape.sample_area(&sampler.get_2d());

        let to_light = sample.sp.p - intersection.p;
        let squared_distance = to_light.squared_length();
        if squared_distance == 0.0 {
            return None;
        }

        let wi = to_light.normalized();

        // The direction the radiance leaves the source in: away from it, back towards the shaded
        // point. `Emitter::l` is stated in those terms, and handing it `wi` — which points the
        // other way — would ask about the face opposite the one being lit.
        let w = -wi;
        let cos_theta_l = vector3::dot(&sample.sp.n, &w);
        if cos_theta_l <= 0.0 {
            return None;
        }

        // (2)
        let pdf = sample.pdf * squared_distance / cos_theta_l;
        if !pdf.is_finite() || pdf <= 0.0 {
            return None;
        }

        let light_sample = LightLiSample {
            spectrum: self.emitter.l(&sample.sp, &w),
            wi,
            pdf,
        };

        Some((light_sample, VisibilityTester::between(&intersection.p, &sample.sp.p)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::colors;
    use crate::geom::vector2::Vector2u;
    use crate::geom::vector3::Vector3f;
    use crate::materials::{DiffuseLight, Material};
    use crate::samplers::IndependentSampler;
    use crate::shapes::{Rectangle, Shape};
    use crate::spectrum::Spectrum;
    use crate::textures::PlainColor;
    use std::f64::consts::PI;

    /// Half-sides of the source, so it spans 2·A by 2·B in the y = 0 plane, facing `+y`.
    const A: f64 = 1.5;
    const B: f64 = 2.5;
    const AREA: f64 = 4.0 * A * B;

    /// Where the shaded point sits on the axis of the source, on the lit side.
    const DISTANCE: f64 = 3.0;

    const EMITTED: Spectrum = colors::WHITE;

    fn area_light() -> AreaLight {
        let shape: Arc<dyn Shape> = Arc::new(Rectangle::new(2.0 * A, 2.0 * B));
        let material = Arc::new(DiffuseLight::new(Arc::new(PlainColor::new(EMITTED))));

        AreaLight::new(shape.area_sampler().unwrap(), material.emitter().unwrap())
    }

    /// A shaded point at `(0, y, 0)`, on the axis of the source. Only `p` is read.
    fn shaded_at(y: f64) -> Intersection {
        Intersection {
            p: Vector3f::new(0.0, y, 0.0),
            d: 1.0,
            n: Vector3f::new(0.0, -1.0, 0.0),
            wo: Vector3f::new(0.0, 1.0, 0.0),
            u: 0.0,
            v: 0.0,
            dpdu: Vector3f::new(1.0, 0.0, 0.0),
            dpdv: Vector3f::new(0.0, 0.0, 1.0),
        }
    }

    fn sampler() -> IndependentSampler {
        // A fixed seed, so these tests either pass or fail, always the same way.
        IndependentSampler::new(0, &Vector2u::new(0, 0), 0)
    }

    /// The density in solid angle, for a point drawn dead centre and seen face on, is `d²/area` —
    /// small enough to work out by hand, which is the point.
    ///
    /// `u = (0.5, 0.5)` lands on the centre of the rectangle, so the direction is the axis, `cos θₗ`
    /// is exactly 1, and [2] reduces to `(1/area) · d²`. A jacobian written upside down, or a `d²`
    /// left out, moves this number and nothing else about the image.
    #[test]
    fn test_the_density_in_solid_angle_is_the_area_density_times_the_jacobian() {
        let light = area_light();
        let mut sampler = FixedPair::new(0.5, 0.5);

        let (sample, _) = light.sample_li(&shaded_at(DISTANCE), &mut sampler).unwrap();

        let expected = DISTANCE * DISTANCE / AREA;
        assert!(
            (sample.pdf - expected).abs() < 1e-12,
            "density is {} where d²/area is {}",
            sample.pdf,
            expected
        );
        assert_eq!(sample.wi, Vector3f::new(0.0, -1.0, 0.0), "the axis points down at the source");
        assert_eq!(sample.spectrum, EMITTED, "seen from the lit face");
    }

    /// `Σ 1/pdf / N` converges to the **solid angle the source subtends**, which has a closed form.
    ///
    /// This is the counterpart, one measure up, of the area conservation test on `sample_area`, and
    /// it is the one that checks the jacobian as a whole. Substituting [2],
    ///
    /// ```text
    /// 1/p(ω) = cos θₗ / (p(A) · d²) = area · cos θₗ / d²
    /// ```
    ///
    /// so the average over uniformly drawn points estimates `∫ cos θₗ/d² dA`, which is the
    /// definition of the solid angle. For a rectangle of half-sides `a` and `b` seen from a point
    /// at distance `d` on its axis, that integral is
    ///
    /// ```text
    /// Ω = 4·atan( a·b / (d·√(a² + b² + d²)) )
    /// ```
    ///
    /// A missing `d²` or a missing cosine changes this number; the previous test, taken alone,
    /// would not see a cosine applied in the wrong place, because its cosine is 1.
    #[test]
    fn test_the_estimator_measures_the_solid_angle_the_source_subtends() {
        let light = area_light();
        let mut sampler = sampler();
        let shaded = shaded_at(DISTANCE);

        const SAMPLES: usize = 200_000;
        let mut total = 0.0;
        for _ in 0..SAMPLES {
            let (sample, _) = light.sample_li(&shaded, &mut sampler).unwrap();
            total += 1.0 / sample.pdf;
        }

        let estimate = total / SAMPLES as f64;
        let exact = 4.0 * (A * B / (DISTANCE * (A * A + B * B + DISTANCE * DISTANCE).sqrt())).atan();
        assert!(
            (estimate / exact - 1.0).abs() < 0.01,
            "solid angle estimated at {} against an exact {}",
            estimate,
            exact
        );
        assert!(exact < 2.0 * PI, "a surface cannot subtend more than a hemisphere");
    }

    /// A point on the unlit face gets no sample at all — not a sample carrying black.
    ///
    /// The source faces `+y`, so a point below it is behind it. Were this to return a sample, the
    /// integrator would trace a shadow ray to a surface that sends it nothing, and `cos θₗ ≤ 0`
    /// would have gone into the division of [2].
    #[test]
    fn test_a_point_behind_the_source_gets_no_sample() {
        let light = area_light();
        let mut sampler = sampler();

        for _ in 0..100 {
            assert!(light.sample_li(&shaded_at(-DISTANCE), &mut sampler).is_none());
        }
    }

    /// A shaded point sitting exactly on the source has no direction to the source.
    #[test]
    fn test_a_point_on_the_source_gets_no_sample() {
        let light = area_light();
        let mut sampler = FixedPair::new(0.5, 0.5);

        assert!(light.sample_li(&shaded_at(0.0), &mut sampler).is_none());
    }

    /// Hands out one chosen pair, so that a test can name the point it draws instead of taking the
    /// one a stream happens to produce.
    struct FixedPair {
        pair: crate::geom::vector2::Vector2f,
    }

    impl FixedPair {
        fn new(x: f64, y: f64) -> Self {
            Self {
                pair: crate::geom::vector2::Vector2f::new(x, y),
            }
        }
    }

    impl Sampler for FixedPair {
        fn get_1d(&mut self) -> f64 {
            self.pair.x
        }

        fn get_2d(&mut self) -> crate::geom::vector2::Vector2f {
            self.pair
        }
    }
}
