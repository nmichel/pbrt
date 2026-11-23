//! A path tracer that reaches every source by two routes, and weighs them so they add up to one.
//!
//! Reference: PBR Book, 4ed, §13.4 — *A Better Path Tracer*, and §2.2.3 — *Multiple Importance
//! Sampling*, which states the estimator below; E. Veach, *Robust Monte Carlo Methods for Light
//! Transport Simulation*, PhD thesis, Stanford, 1997, chapter 9, where it is introduced and proved.
//! <https://pbr-book.org/4ed/Light_Transport_I_Surface_Reflection/A_Better_Path_Tracer>
//! <https://pbr-book.org/4ed/Monte_Carlo_Integration/Improving_Efficiency#MultipleImportanceSampling>
//!
//! # Frame
//!
//! Every density in this file is **per unit solid angle at the vertex the direction leaves from**,
//! in world space: [`Material::pdf`](crate::materials::Material::pdf) and
//! [`Light::pdf_li`] are both stated in that measure, and so is the `density` of the samples they
//! draw. The weights below compare densities, and comparing two that are not in one measure would
//! weigh one strategy by a jacobian the other does not have.
//!
//! # Two strategies for one integral
//!
//! At a vertex `x`, the light arriving directly from the sources is
//!
//! ```text
//!   L_d(x, wo) = ∫ f(x, wo, wi) · L_e(wi) · |cos θᵢ| dωᵢ
//! ```
//!
//! and the path tracer has two ways of drawing a `wi` for it:
//!
//! - **next event estimation** — pick a source `l` with probability `P(l)`, then draw `wi` towards
//!   it with density `p_l(wi)`. The strategy's density over directions is `p_L(wi) = P(l) · p_l(wi)`;
//!   it is good where the source is small and the bsdf broad;
//! - **bsdf sampling** — draw `wi` from the bsdf with density `p_f(wi)`, continue the path, and
//!   collect `L_e` if the ray reaches an emitter. It is good where the bsdf is narrow and the source
//!   large.
//!
//! Either one alone is unbiased. Adding both would count the direct light twice.
//!
//! # The multi-sample estimator
//!
//! Draw one `X_L` by the first strategy and one `X_f` by the second, and form
//!
//! ```text
//!   F = w_L(X_L) · g(X_L) / p_L(X_L)  +  w_f(X_f) · g(X_f) / p_f(X_f)        g = f · L_e · |cos θ|
//! ```
//!
//! Its expectation is `∫ (w_L + w_f) · g dω` — each term's density cancels against its own
//! division — so **F is unbiased exactly when, for every direction where g ≠ 0,**
//!
//! ```text
//!   [1]  w_L(ω) + w_f(ω) = 1                                    and  wᵢ(ω) = 0 where pᵢ(ω) = 0
//! ```
//!
//! [1] constrains nothing else, and the choice of weights is free. The **power heuristic** with
//! exponent β = 2 is the one Veach shows rarely loses to any other (thesis, §9.2.4):
//!
//! ```text
//!   [2]  w_s(ω) = p_s(ω)² / (p_L(ω)² + p_f(ω)²)
//! ```
//!
//! which satisfies [1] by construction — the two numerators are the denominator. Whichever strategy
//! is the more likely to produce `ω` takes the larger share, and where one of them cannot produce
//! it at all the other takes everything. Both densities enter both weights, and this is why each
//! participant had to learn to state the density *the other* would have drawn with:
//! [`Light::pdf_li`] for a direction the bsdf drew, `Material::pdf` for a direction a light drew.
//!
//! # Where the δ distributions go
//!
//! A δ has no density to compare: [`DirectionDensity::Dirac`]. It is still covered by [2], as its
//! limit. A direction a δ light produces has `p_f = 0` against `p_L = ∞` — no bsdf with a density
//! ever draws it — and a direction a δ bsdf produces has `p_L = 0` against `p_f = ∞`. In both cases
//! the δ strategy takes the whole weight, and that is what the code does: a Dirac sample is
//! weighted 1, without calling [2]. The same goes for the camera ray, which no light strategy
//! drew, and for an emitter that no source samples ([`Interaction::area_light`] is `None`).
//!
//! This is what replaces the rule "count the emission only after a specular bounce". That rule
//! was the δ case of [2] written out by hand, with the non-δ case set to `w_f = 0` — correct, and
//! the reason a large source seen through a broad bsdf came out noisy.
//!
//! # What the vertex keeps between two iterations
//!
//! The emission found by the bsdf strategy is collected one iteration *later*, at the next hit,
//! but its weight is a property of the vertex the ray left from: `pdf_li` is asked from that
//! vertex, and `p_f` is the density the ray was drawn with there. [`BsdfDraw`] carries the two
//! across.

use crate::colors;
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::lights::{Light, LightType};
use crate::pdfs::DirectionDensity;
use crate::samplers::Sampler;
use crate::scene::Scene;
use crate::spectrum::Spectrum;

use super::Integrator;

pub struct PathIntegrator {
    /// Max recursion depth
    pub max_depth: usize,
}

struct SampledLight<'a> {
    /// The light source
    pub light: &'a dyn Light,

    /// The light's discrete probability of been sampled
    pub p: f64,
}

/// How the direction of the ray being followed was drawn — what the emission it finds is weighed
/// against.
enum Arrival {
    /// No light strategy could have drawn this direction: the camera ray, or a δ bounce. Emission
    /// found along it takes the whole weight.
    Unweighted,

    /// Drawn by a bsdf with a density, `p_f` of [2].
    BsdfDraw(BsdfDraw),
}

struct BsdfDraw {
    /// Where the competing light strategy is asked from: **the origin of the ray that was traced**,
    /// not the shaded point.
    ///
    /// The two differ by the anti-acne offset a material applies before sending its ray, and the
    /// difference is not negligible where it matters. [`Light::pdf_li`] finds the point a
    /// direction reaches by intersecting the source again, so it has to retrace the ray that found
    /// it: from the shaded point instead, a grazing direction that clipped the edge of a source
    /// misses it, `pdf_li` answers 0 for a direction the bsdf ray did reach, and the bsdf strategy
    /// takes the whole weight of an emission next event estimation was also counting. On
    /// `test_files/grazing_source.stage`, a 0.001 offset at 6° of elevation moves the hit by about
    /// a hundredth — enough for one path in a hundred thousand to carry the full ρ·L of a source of
    /// radiance 600, into a point whose radiance is 0.48.
    from: Vector3f,

    pdf: f64,
}

/// The power heuristic, [2] of the module header: the weight of a strategy that drew its
/// direction with density `p_s`, against one that would have drawn it with density `p_other`.
///
/// The two calls `power_heuristic(a, b)` and `power_heuristic(b, a)` add up to 1, which is [1];
/// a test below checks it.
///
/// Both densities are 0 only for a direction neither strategy produces, whose `g` is 0 too — a
/// bsdf whose density vanishes where it does not reflect, a light that answers 0 where it does not
/// emit. The weight is then indeterminate and its value irrelevant; 0 keeps a `NaN` out of the
/// image.
fn power_heuristic(p_s: f64, p_other: f64) -> f64 {
    let p_s_squared = p_s * p_s;
    let sum_of_squares = p_s_squared + p_other * p_other;
    if sum_of_squares == 0.0 {
        return 0.0;
    }
    p_s_squared / sum_of_squares
}

impl PathIntegrator {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }

    /// `P(l)` of the module header: the chance next event estimation picks a given source.
    ///
    /// The pick is uniform, so this is the same for every source — and it must stay in step with
    /// [`Self::sample_light`], since a bsdf hit asks it for the source the path happened to reach.
    fn light_selection_probability(scene: &Scene) -> f64 {
        1.0 / scene.get_light_count() as f64
    }

    fn sample_light<'a>(&self, scene: &'a Scene, sampler: &mut dyn Sampler) -> Option<SampledLight<'a>> {
        let light_counts = scene.get_light_count();

        if light_counts == 0 {
            return None;
        }

        let light_index = (sampler.get_1d() * light_counts as f64).min(light_counts as f64 - 1.0) as usize;
        let light_ref = scene.get_light_at(light_index).unwrap();
        Some(SampledLight {
            light: light_ref,
            p: Self::light_selection_probability(scene),
        })
    }

    /// The weight of emission from `light`, found along a ray that arrived by `arrival` in
    /// direction `wi` — `w_f` of [2].
    fn bsdf_strategy_weight(scene: &Scene, arrival: &Arrival, light: &dyn Light, wi: &Vector3f) -> f64 {
        match arrival {
            Arrival::Unweighted => 1.0,
            Arrival::BsdfDraw(BsdfDraw { from, pdf }) => {
                let p_light = Self::light_selection_probability(scene) * light.pdf_li(from, wi);
                power_heuristic(*pdf, p_light)
            }
        }
    }
}

impl Integrator for PathIntegrator {
    fn li(&self, ray: &Ray, scene: &Scene, _depth: usize, near: f64, far: f64, sampler: &mut dyn Sampler) -> Spectrum {
        let mut beta: Spectrum = colors::WHITE;
        let mut accumulated_radiance: Spectrum = colors::BLACK;
        let mut current_ray: Ray = ray.clone();
        let mut arrival = Arrival::Unweighted;
        let mut bounce_count = 0;

        loop {
            if let Some(interaction) = scene.intersect(&current_ray, near, far) {
                let Interaction { ref material, .. } = interaction;
                if let Some(emitted) = material.emit(&current_ray, &interaction) {
                    // The bsdf strategy's share of this emission. A surface no source samples has
                    // no competitor, and keeps all of it.
                    let weight = match interaction.area_light {
                        Some(ref light) => Self::bsdf_strategy_weight(scene, &arrival, light.as_ref(), &current_ray.direction),
                        None => 1.0,
                    };
                    accumulated_radiance += emitted * &beta * weight;
                }

                bounce_count += 1;
                if bounce_count == self.max_depth {
                    break;
                }

                // Sample direct illumination
                let wo = -current_ray.direction;
                if let Some(ref sampled_light) = self.sample_light(scene, sampler) {
                    if let Some((ref sample_li, ref visibility_tester)) = sampled_light.light.sample_li(&interaction.intersection, sampler) {
                        let wi = &sample_li.wi;
                        let f = material.f(&wo, wi, &interaction) * vector3::abs_dot(wi, &interaction.intersection.n);
                        if visibility_tester.unoccluded(scene) {
                            // What this sample is divided by — the chance of having picked this
                            // source, times the density of `wi` if it has one — and the light
                            // strategy's share of it, `w_L` of [2]. A δ source has no density, its
                            // `spectrum` being the integral over directions already, and no bsdf
                            // could have found it: it keeps the whole weight.
                            let (sample_probability, weight) = match sample_li.density {
                                DirectionDensity::SolidAngle(pdf) => {
                                    let p_light = sampled_light.p * pdf;
                                    let p_bsdf = material.pdf(&wo, wi, &interaction);
                                    (p_light, power_heuristic(p_light, p_bsdf))
                                }
                                DirectionDensity::Dirac => (sampled_light.p, 1.0),
                            };
                            let light_contribution = &sample_li.spectrum * &f * &beta * weight / sample_probability;
                            accumulated_radiance += light_contribution;
                        }
                    }
                }

                if let Some(ref scatter_info) = material.scatter(&current_ray, &interaction, sampler) {
                    // Sample outgoing direction to continue the path
                    beta *= match scatter_info.density {
                        DirectionDensity::SolidAngle(pdf) => {
                            let abs_cos_theta = vector3::abs_dot(&scatter_info.scattered.direction, &interaction.intersection.n);
                            scatter_info.attenuation * abs_cos_theta / pdf
                        }
                        // A δ has already integrated over the projected solid angle: the attenuation is
                        // the fraction of energy carried along the one direction, with no cosine to
                        // apply and no density to divide by.
                        DirectionDensity::Dirac => scatter_info.attenuation,
                    };
                    arrival = match scatter_info.density {
                        DirectionDensity::SolidAngle(pdf) => {
                            Arrival::BsdfDraw(BsdfDraw {
                                from: scatter_info.scattered.origin,
                                pdf,
                            })
                        }
                        DirectionDensity::Dirac => Arrival::Unweighted,
                    };
                    current_ray = scatter_info.scattered.clone();
                }
                else {
                    break; // exit the loop if we hit a non-scattering material
                }

                /*
                // Possibly terminate the path with Russian roulette
                // q is the probability of continuing the path
                let q = beta.max_component_value();
                if sampler.get_1d() > q {
                    break; // terminate the path
                }

                beta *= 1.0 / q; // scale beta to account for the probability of continuing
                */
            }
            else {
                // The ray leaves the scene, and reaches every infinite light at once. Each is
                // weighed on its own, for each is a source next event estimation could have
                // picked, with its own density for this direction. That is why this does not
                // go through `background_radiance`, which returns their sum.
                for light in scene.query_lights(&LightType::Infinite) {
                    let weight = Self::bsdf_strategy_weight(scene, &arrival, light, &current_ray.direction);
                    accumulated_radiance += light.le(&current_ray) * &beta * weight;
                }
                break;
            }
        }

        accumulated_radiance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    use crate::geom::transform::Transform;
    use crate::geom::vector2::Vector2u;
    use crate::lights::{AreaLight, UniformInfiniteLight};
    use crate::materials::{DiffuseLight, Lambertian, Material};
    use crate::objects::Simple;
    use crate::samplers::IndependentSampler;
    use crate::shapes::{self, Plane, Shape, Sphere};
    use crate::textures::PlainColor;

    /// [1] of the module header, for the heuristic [2] as written: the two weights of a direction
    /// add up to 1 whatever the two densities, including when one of them is 0.
    #[test]
    fn test_the_two_weights_add_up_to_one() {
        let densities = [0.0, 1e-6, 0.1, 1.0 / (4.0 * std::f64::consts::PI), 1.0, 37.0, 1e6];
        for &p_light in densities.iter() {
            for &p_bsdf in densities.iter() {
                if p_light == 0.0 && p_bsdf == 0.0 {
                    continue;
                }
                let sum = power_heuristic(p_light, p_bsdf) + power_heuristic(p_bsdf, p_light);
                assert!(
                    (sum - 1.0).abs() < 1e-12,
                    "p_L = {}, p_f = {}: the weights add up to {}",
                    p_light,
                    p_bsdf,
                    sum
                );
            }
        }
    }

    /// A strategy that cannot produce a direction gets no share of it — the second half of [1].
    #[test]
    fn test_a_strategy_that_cannot_draw_a_direction_has_no_weight() {
        assert_eq!(power_heuristic(0.0, 2.0), 0.0);
        assert_eq!(power_heuristic(2.0, 0.0), 1.0);
    }

    const ALBEDO: f64 = 0.5;

    fn grey_floor() -> Arc<dyn Material> {
        Arc::new(Lambertian::new(Arc::new(PlainColor::new(Spectrum::new(ALBEDO, ALBEDO, ALBEDO)))))
    }

    /// The mean of `li` over `n` paths, all starting with a ray straight down onto the floor y = 0
    /// at the origin, from half a unit above it — under whatever lamp the scene hangs higher up.
    ///
    /// `max_depth` is 2: one vertex on the floor, where next event estimation runs, and one more
    /// for the bsdf ray to find what it reaches. Both strategies estimate the same direct light,
    /// and nothing else is collected — the floor sees no other surface. The floor is grey, so any
    /// channel reads the same.
    fn mean_radiance_at_the_floor(scene: &Scene, n: usize) -> f64 {
        let integrator = PathIntegrator::new(2);
        let looking_down = Ray::new(&Vector3f::new(0.0, 0.5, 0.0), &Vector3f::new(0.0, -1.0, 0.0));

        let mut total = 0.0;
        for index in 0..n {
            let mut sampler = IndependentSampler::new(0, &Vector2u::new(0, 0), index);
            total += integrator.li(&looking_down, scene, 0, 0.0001, 1000.0, &mut sampler).max_component_value();
        }
        total / n as f64
    }

    /// Under a uniform sky of radiance L, a lambertian of albedo ρ sends back exactly ρ·L.
    ///
    /// `L_o = ∫ (ρ/π) · L · cos θ dω = (ρ/π) · L · π = ρ·L`, over the hemisphere above the floor.
    ///
    /// The sky is reached by both strategies — next event estimation draws it on the whole sphere,
    /// the bsdf ray escapes into it — so this is a test of [1] at the scale of the integrator. Were
    /// the two contributions added unweighted the mean would come out at 2ρ·L; were one of them
    /// dropped by mistake it would still come out at ρ·L, which is why the next test exists.
    #[test]
    fn test_a_floor_under_a_uniform_sky_reflects_its_albedo() {
        let radiance = 1.0;
        let mut scene = Scene::new();
        scene.add_object(Arc::new(Simple::new(Arc::new(Plane::new()), grey_floor())));
        scene.add_light(Arc::new(UniformInfiniteLight::new(Spectrum::new(radiance, radiance, radiance))));
        scene.commit();

        let mean = mean_radiance_at_the_floor(&scene, 20_000);

        let expected = ALBEDO * radiance;
        assert!((mean - expected).abs() < 0.01 * expected, "mean {} against ρ·L = {}", mean, expected);
    }

    /// Under a sphere of radiance L seen within a cone of half-angle α, a lambertian of albedo ρ
    /// sends back ρ·L·sin²α — and both strategies are needed to get there.
    ///
    /// The sphere stands straight above the shaded point, so the cone is centred on the normal and
    /// the integral is
    ///
    /// ```text
    ///   L_o = (ρ/π) · L · ∫₀^{2π} ∫₀^α cos θ sin θ dθ dφ = (ρ/π) · L · π sin²α = ρ·L·sin²α
    /// ```
    ///
    /// with `sin α = r / h` for a sphere of radius `r` whose centre is at height `h`.
    ///
    /// Unlike the sky, this source is reached by the bsdf ray only through the [`AreaLight`] the
    /// hit names: a weight read off the wrong source, or not read at all, leaves the two shares not
    /// adding up. The bsdf strategy's share is the smaller one here, a few percent of the answer —
    /// next event estimation draws within the very cone that is being integrated — and the bound
    /// is set below it, so the test fails if that route is lost.
    #[test]
    fn test_a_floor_under_a_spherical_lamp_reflects_the_cone_it_sees() {
        let radiance = 4.0;
        let (r, h) = (1.0, 2.0);

        let to_world = Box::new(Transform::translation(Vector3f::new(0.0, h, 0.0)));
        let lamp_shape: Arc<dyn Shape> = Arc::new(shapes::Transformed::new(Arc::new(Sphere::new(r)), to_world));
        let lamp_material: Arc<dyn Material> = Arc::new(DiffuseLight::new(Arc::new(PlainColor::new(Spectrum::new(radiance, radiance, radiance)))));
        let emitter = Arc::clone(&lamp_material).emitter().unwrap();
        let lamp: Arc<dyn Light> = Arc::new(AreaLight::new(Arc::clone(&lamp_shape).area_sampler().unwrap(), emitter));

        let mut scene = Scene::new();
        scene.add_object(Arc::new(Simple::new(Arc::new(Plane::new()), grey_floor())));
        scene.add_object(Arc::new(Simple::emitting(lamp_shape, lamp_material, Arc::clone(&lamp))));
        scene.add_light(lamp);
        scene.commit();

        let mean = mean_radiance_at_the_floor(&scene, 20_000);

        let sin_alpha = r / h;
        let expected = ALBEDO * radiance * sin_alpha * sin_alpha;
        assert!(
            (mean - expected).abs() < 0.01 * expected,
            "mean {} against ρ·L·sin²α = {}",
            mean,
            expected
        );
    }

    /// The share of emission the bsdf strategy takes is read off the ray that was traced, not off
    /// one rebuilt from the shaded point — [`BsdfDraw::from`].
    ///
    /// A long strip of radiance 600, 2 mm deep, hangs 0.35 above the floor and as far to the side,
    /// so it is seen at 45°. Next event estimation is by far the likelier strategy for so thin a
    /// source, and a bsdf ray that hits it is given a weight below 10⁻³. Were the competing density
    /// asked from the shaded point instead of the ray's offset origin, the retraced ray would land
    /// 1 mm further on — beyond the far edge for half the hits: weight 1, and a single path
    /// carrying the whole ρ·L = 300. Next event estimation alone never brings more than about 8
    /// into this point, which is what the bound is set against.
    ///
    /// The geometry is chosen to make the failure likely rather than rare: the misplaced band is
    /// `offset / tan(elevation)` deep, so it shrinks at grazing angles, and the chance of a bsdf
    /// ray reaching it peaks around 45°.
    #[test]
    fn test_a_bsdf_ray_reaching_a_thin_source_takes_almost_none_of_it() {
        let radiance = 600.0;
        let depth = 0.002;

        let strip_to_world = Transform::from_matrix(
            &crate::geom::matrix4::Matrix4::translation(0.0, 0.35, 0.35) * &crate::geom::matrix4::Matrix4::rotation_x(std::f64::consts::PI),
        );
        let strip_shape: Arc<dyn Shape> = Arc::new(shapes::Transformed::new(
            Arc::new(shapes::Rectangle::new(20.0, depth)),
            Box::new(strip_to_world),
        ));
        let strip_material: Arc<dyn Material> = Arc::new(DiffuseLight::new(Arc::new(PlainColor::new(Spectrum::new(radiance, radiance, radiance)))));
        let emitter = Arc::clone(&strip_material).emitter().unwrap();
        let strip: Arc<dyn Light> = Arc::new(AreaLight::new(Arc::clone(&strip_shape).area_sampler().unwrap(), emitter));

        let mut scene = Scene::new();
        scene.add_object(Arc::new(Simple::new(Arc::new(Plane::new()), grey_floor())));
        scene.add_object(Arc::new(Simple::emitting(strip_shape, strip_material, Arc::clone(&strip))));
        scene.add_light(strip);
        scene.commit();

        let integrator = PathIntegrator::new(2);
        let looking_down = Ray::new(&Vector3f::new(0.0, 0.2, 0.0), &Vector3f::new(0.0, -1.0, 0.0));
        let mut brightest: f64 = 0.0;
        for index in 0..20_000 {
            let mut sampler = IndependentSampler::new(0, &Vector2u::new(0, 0), index);
            brightest = brightest.max(
                integrator
                    .li(&looking_down, &scene, 0, 0.0001, 1000.0, &mut sampler)
                    .max_component_value(),
            );
        }

        let full_share = ALBEDO * radiance;
        assert!(
            brightest < 0.1 * full_share,
            "a path carried {} — a bsdf hit took the whole of ρ·L = {}",
            brightest,
            full_share
        );
    }
}
