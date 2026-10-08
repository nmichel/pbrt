use crate::colors;
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::interaction::Interaction;
use crate::materials::{ScatterDensity, ScatterInfo};
use crate::samplers::Sampler;
use crate::scene::Scene;
use crate::spectrum::Spectrum;

use super::Integrator;

pub struct NaiveIntegrator {
    /// Max recursion depth
    pub max_depth: usize,
}

impl NaiveIntegrator {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }
}

impl Integrator for NaiveIntegrator {
    fn li(&self, ray: &Ray, scene: &Scene, depth: usize, near: f64, far: f64, sampler: &mut dyn Sampler) -> Spectrum {
        if depth <= 0 {
            return colors::BLACK;
        }
        match scene.intersect(&ray, near, far) {
            Some(interaction) => {
                let Interaction { ref material, .. } = interaction;
                let emitted = match material.emit(&ray, &interaction) {
                    Some(emitted) => emitted,
                    None => Spectrum::new(0.0, 0.0, 0.0),
                };
                match material.scatter(ray, &interaction, sampler) {
                    Some(ScatterInfo {
                        attenuation,
                        scattered,
                        density,
                    }) => {
                        let li = self.li(&scattered, scene, depth - 1, near, far, sampler);
                        match density {
                            ScatterDensity::SolidAngle(pdf) => {
                                let abs_cos = vector3::abs_dot(&scattered.direction, &interaction.intersection.n);
                                emitted + attenuation * &li * abs_cos / pdf
                            }
                            // A δ has already integrated over the projected solid angle: the attenuation
                            // is the fraction of energy carried along the one direction, with no cosine
                            // to apply and no density to divide by.
                            ScatterDensity::Dirac => emitted + attenuation * &li,
                        }
                    }
                    None => emitted,
                }
            }
            None => self.background_radiance(&ray, &scene),
        }
    }
}
