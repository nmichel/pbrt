use crate::geom::intersectable::{Intersectable, Intersection};
use crate::geom::ray::Ray;
use crate::geom::vector3;
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
        let intersection_opt = scene.intersect(&ray);
        match intersection_opt {
            Some(intersection) => {
                //let s = vector3::dot(&wo, &n);
                //Spectrum::new(s, s, s);

                let Intersection { ref n, ref wo, .. } = intersection;
                let l = Spectrum::new(0.0, 0.0, 0.0);
                // L += isect.Le(wo); // Dans le cas ou le material est un emitter
                scene.lights.iter().fold(l, |acc, light| {
                    let (ref li, ref wi, ref tester) = light.li(&intersection);
                    // Spectrum f = isect.bsdf->f(wo, wi);
                    let f = Spectrum::new(1.0, 0.0, 0.0); // NOTE : HARDCODED RED Spectrum for every object !
                    if tester.unoccluded(&scene) {
                        acc + f * li * vector3::dot(&wi, n).abs()
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
