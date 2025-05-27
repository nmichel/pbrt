use crate::colors;
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::interaction::{self, Interaction};
use crate::lights::{Light, LightType, UniformInfiniteLight};
use crate::materials::ScatterInfo;
use crate::scene::Scene;
use crate::spectrum::Spectrum;
use crate::utils::random_double;

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

impl PathIntegrator {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }

    fn sample_light<'a>(&self, scene: &'a Scene, _interaction: &Interaction) -> Option<SampledLight<'a>> {
        let light_counts = scene.get_light_count();

        if light_counts == 0 {
            return None;
        }

        let light_index = (random_double() * light_counts as f64).min(light_counts as f64 - 1.0) as usize;
        let light_ref = scene.get_light_at(light_index).unwrap();
        Some(SampledLight {
            light: light_ref,
            p: 1.0 / light_counts as f64,
        })
    }
}

impl Integrator for PathIntegrator {
    fn li(&self, ray: &Ray, scene: &Scene, depth: usize, near: f64, far: f64) -> Spectrum {
        let mut beta: Spectrum = colors::WHITE;
        let mut accumulated_radiance: Spectrum = colors::BLACK;
        let mut current_ray: Ray = ray.clone();
        let mut is_last_bounce_specular: bool = true;
        let mut bounce_count = 0;

        loop {
            if let Some(interaction) = scene.intersect(&current_ray, near, far) {
                let Interaction { ref material, .. } = interaction;
                if is_last_bounce_specular {
                    if let Some(emitted) = material.emit(&current_ray, &interaction) {
                        accumulated_radiance += emitted * &beta;
                    }
                }

                bounce_count += 1;
                if bounce_count == self.max_depth {
                    break;
                }

                // Sample direct illumination
                if let Some(ref sampled_light) = self.sample_light(scene, &interaction) {
                    if let Some((ref sample_li, ref visibility_tester)) = sampled_light.light.sample_li(&interaction.intersection) {
                        let wi = &sample_li.wi;
                        let f = material.f(&-current_ray.direction, wi, &interaction) * vector3::dot(wi, &interaction.intersection.n).abs();
                        if visibility_tester.unoccluded(scene) {
                            let light_contribution = &sample_li.spectrum * &f * &beta / (sampled_light.p * sample_li.pdf);
                            accumulated_radiance += light_contribution;
                        }
                    }
                }

                if let Some(ref scatter_info) = material.scatter(&current_ray, &interaction) {
                    // Sample outgoing direction to continue the path
                    let abs_cos_theta = vector3::dot(&scatter_info.scattered.direction, &interaction.intersection.n).abs();
                    beta *= scatter_info.attenuation * abs_cos_theta / scatter_info.pdf;
                    current_ray = scatter_info.scattered.clone();
                    is_last_bounce_specular = material.is_specular();
                }
                else {
                    break; // exit the loop if we hit a non-scattering material
                }

                /*
                // Possibly terminate the path with Russian roulette
                // q is the probability of continuing the path
                let q = beta.max_component_value();
                if random_double() > q {
                    break; // terminate the path
                }

                beta *= 1.0 / q; // scale beta to account for the probability of continuing
                */
            }
            else {
                if is_last_bounce_specular {
                    // The last bounce was specular, so lights have not been sampled
                    // so we need to add the background radiance at this very last step.
                    // cf. https://pbr-book.org/4ed/Light_Transport_I_Surface_Reflection/A_Simple_Path_Tracer
                    accumulated_radiance += self.background_radiance(&current_ray, scene) * &beta;
                }
                break;
            }
        }

        accumulated_radiance
    }

    fn background_radiance(&self, ray: &Ray, scene: &Scene) -> Spectrum {
        let infinite_lights = scene.query_lights(&LightType::Infinite);
        let mut res: Spectrum = colors::BLACK;
        for light in infinite_lights {
            res += light.le(ray);
        }
        res
    }
}
