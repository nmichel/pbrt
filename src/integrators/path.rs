use std::f64::consts::PI;

use crate::{colors, interaction};
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::interaction::Interaction;
use crate::materials::ScatterInfo;
use crate::scene::Scene;
use crate::spectrum::Spectrum;

use super::Integrator;

pub struct PathIntegrator {
    /// Max recursion depth
    pub max_depth: usize,
}

impl PathIntegrator {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }

    fn background_radiance(&self, ray: &Ray, _scene: &Scene) -> Spectrum {
        let mut unit_direction = ray.direction;
        unit_direction.normalize();
        let t = 0.5 * (unit_direction.y + 1.0);
        return Spectrum::new(1.0, 1.0, 1.0) * (1.0 - t) + Spectrum::new(0.5, 0.7, 1.0) * t;
    }
}

impl Integrator for PathIntegrator {
    fn li(&self, ray: &Ray, scene: &Scene, depth: usize, near: f64, far: f64) -> Spectrum {
        let mut beta : Spectrum = colors::WHITE;
        let mut accumulated_radiance: Spectrum = colors::BLACK;
        let mut current_ray: Ray = ray.clone();

        for _bounces in 0..self.max_depth {
            if let Some(interaction) = scene.intersect(&current_ray, near, far) {
                let Interaction { ref material, .. } = interaction;
                if let Some(emitted) = material.emit(&current_ray, &interaction) {
                    accumulated_radiance += emitted * &beta;
                }

                if let Some(ScatterInfo { attenuation, ref scattered, pdf }) = material.scatter(&current_ray, &interaction) {
                    let abs_cos_theta = vector3::dot(&scattered.direction, &interaction.intersection.n).abs();
                    let weight = attenuation * abs_cos_theta / pdf;
                    beta *= &weight;
                    current_ray = scattered.clone();
                }
                else {
                    break; // exit the loop if we hit a non-scattering material
                }
            }
            else {
                accumulated_radiance += self.background_radiance(&current_ray, scene) * &beta;
                break;
            }
        }

        accumulated_radiance

        /*
        let mut l : Spectrum = colors::BLACK;
        let mut beta : Spectrum = colors::WHITE;
        let mut current_ray : Ray = ray.clone();
        let mut is_specular_bounce : bool = false;
        let mut bounce = 0;

        // print!("* LI \n");
        loop {
            // print!("beta: {:?}\n", &beta);

            let intersection: Option<Interaction> = scene.intersect(&ray, near, far);
            if bounce == 0 || is_specular_bounce {
                match intersection {
                    Some(ref interaction) => {
                        let Interaction { ref material, .. } = interaction;
                        let emitted = match material.emit(&ray, &interaction) {
                            Some(emitted) => emitted,
                            None => Spectrum::new(0.0, 0.0, 0.0),
                        };
                        l += emitted * &beta;
                    }
                    None => {
                        l += self.background_radiance(&current_ray, scene) * &beta;
                    }
                }
            }
            else {
                l += self.background_radiance(&current_ray, scene) * &beta;
            }

            if bounce >= self.max_depth || intersection.is_none() {
                break;
            }

            let interaction = intersection.unwrap();
            let Interaction { ref material, .. } = interaction;

            if ! material.is_specular() {
                let nb_light = 1;
                let light_pdf : f64 = 1.0 / nb_light as f64;
                let light = scene.get_light_at(0).unwrap();
    
                let (li, wi, visibility_tester) = light.li(&interaction.intersection);
                if li != colors::BLACK {
                    // let f = material.bsdf(&interaction, &wi, &ray.direction);
                    // let pdf = material.pdf(&wi, &ray.direction, &interaction);
                    // let pdf = pdf * light_pdf;
                    // let pdf = pdf.max(0.0001);

                    // let cos_theta = vector3::dot(&wi, &interaction.intersection.n).abs();
                    // let cos_theta = cos_theta.max(0.0001);

                    // let f = f * cos_theta / pdf;
                    // let f = f.max(0.0);

                    let is_visible = visibility_tester.unoccluded(scene);
                    if is_visible {
                        let light_f = material.f(&wi, &ray.direction);
                        let abs_cos_theta = vector3::dot(&wi, &interaction.intersection.n).abs();
                        let light_contribution = li * light_f * abs_cos_theta / light_pdf;
                        l += light_contribution * &beta;
                    }
                }
            }

            is_specular_bounce = material.is_specular();

            match material.scatter(ray, &interaction) {
                Some(ScatterInfo { attenuation, scattered, pdf }) => {
                    let abs_cos = vector3::dot(&scattered.direction, &interaction.intersection.n).abs();
                    beta = beta * &attenuation * abs_cos / pdf;
                    current_ray = scattered;
                }
                None => break,
            }

            bounce += 1;
        };

        l
        */
    }
}
