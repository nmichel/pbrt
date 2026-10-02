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
//! # Where the measure changes, and where it no longer does
//!
//! An integrator works in **solid angle**; a surface is parameterised by **area**. The jacobian
//! between the two densities used to be written out here, and is not any more: it belongs to the
//! shapes, which is where a shape can choose to skip it. [`solid_angle_from_area`] holds the
//! derivation, and [`AreaSampleable::sample_solid_angle`] is what this file asks.
//!
//! What stays here is the one thing a shape cannot answer: **which face emits**. A shape hands
//! back a point whichever side it was drawn on, with a density that is honest either way; it is
//! this file that knows the emitter is one-sided ([`Emitter`]) and drops what faces the wrong way.
//! The two questions used to be answered by one comparison, and separating them is what lets a
//! shape draw in its own solid angle without ever being told about emission.
//!
//! # A departure, now only half of one
//!
//! A [`Sphere`](crate::shapes::Sphere) draws within the cone it subtends, so every one of its
//! samples lands where the light comes from. A [`Rectangle`](crate::shapes::Rectangle) still draws
//! uniformly over its area and converts: unbiased, and noisy when the source is large and seen at
//! a grazing angle, because most of the points then land where they matter least. Which shape does
//! which is now the shape's business, and this file does not change either way.

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
    /// The shape is asked for a point *as seen from* the shaded point, so it comes back with a
    /// density already over directions — by its own draw if it knows one, by the conversion of
    /// [`solid_angle_from_area`](crate::shapes::solid_angle_from_area) otherwise. What this adds is
    /// the emission: which face the point was drawn on, and what it radiates.
    ///
    /// # The two ways there is no sample
    ///
    /// Both answer `None` rather than a sample carrying zero, because a sample worth nothing still
    /// costs a shadow ray.
    ///
    /// - **There is no direction, or no usable density.** The shape says so, and the reasons are
    ///   geometric: the shaded point lies on the surface, or a density came out of a vanishing
    ///   cosine as something that cannot be divided by.
    /// - **The shaded point lies on the unlit face.** This one is not geometry and the shape cannot
    ///   know it: emission is one-sided, so a point whose normal turns away radiates nothing this
    ///   way. A shape that draws in its own solid angle never produces such a point, and the test
    ///   then costs a dot product and never fires.
    fn sample_li(&self, intersection: &Intersection, sampler: &mut dyn Sampler) -> Option<(LightLiSample, VisibilityTester)> {
        let sample = self.shape.sample_solid_angle(&intersection.p, &sampler.get_2d())?;

        // The direction the radiance leaves the source in: away from it, back towards the shaded
        // point. `Emitter::l` is stated in those terms, and handing it `wi` — which points the
        // other way — would ask about the face opposite the one being lit.
        let w = -sample.wi;
        if vector3::dot(&sample.sp.n, &w) <= 0.0 {
            return None;
        }

        let light_sample = LightLiSample {
            spectrum: self.emitter.l(&sample.sp, &w),
            wi: sample.wi,
            pdf: sample.pdf,
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
