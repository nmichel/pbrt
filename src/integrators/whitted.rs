use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::interaction::Interaction;
use crate::materials::material::Material;
use crate::scene::Scene;
use crate::spectrum::Spectrum;

use super::integrator::Integrator;

pub struct WhittedIntegrator {
    // Max recursion depth
    max_depth: usize
}

impl WhittedIntegrator {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }
}

impl Integrator for WhittedIntegrator {
    fn li(&self, ray: &Ray, scene: &Scene, depth: usize) -> Spectrum {
        match scene.intersect(&ray) {
            Some(interaction) => {
                let Interaction { ref intersection, ref material } = interaction;
                let Intersection { ref n, ref wo, .. } = intersection;

                let f = material.shade(&intersection); // isect.bsdf->f(wo, wi);

                // Object may be an emitter
                let mut l = intersection.le(wo);

                // Add ambiant lighting
                l = l + &f * &Spectrum::new(0.1, 0.1, 0.1);

                // Add contribution of each light source
                scene.lights.iter().fold(l, |acc, light| {
                    let (ref li, ref wi, ref tester) = light.li(&intersection);
                    // Spectrum f = isect.bsdf->f(wo, wi);
                    if tester.unoccluded(&scene) {
                        acc + (&f * li) * vector3::dot(&wi, n).abs()
                    }
                    else {
                        acc
                    }
                })
            },
            None => {
                scene.background_radiance(&ray) // In the book, part of the integrator
            }
        }
    }
}
