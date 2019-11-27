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
        match scene.intersect(&ray) {
            Some(intersection) => {
                let Intersection { ref n, ref wo, ref u, ref v, .. } = intersection;

                let Intersection { ref n, ref wo, .. } = intersection;
                let l = Spectrum::new(0.0, 0.0, 0.0);
                // L += isect.Le(wo); // Dans le cas ou le material est un emitter
                scene.lights.iter().fold(l, |acc, light| {
                    let (ref li, ref wi, ref tester) = light.li(&intersection);
                    // Spectrum f = isect.bsdf->f(wo, wi);

                    // NOTE : HARDCODED uv-based checkerboard pattern Spectrum for every object !
                    let scale_u = (u * 10.0) % 1.0;
                    let scale_v = (v * 10.0) % 1.0;
                    let r =
                        if (scale_u < 0.5 && scale_v < 0.5) ||
                           (scale_u >= 0.5 && scale_v >= 0.5) {
                            1.0
                        }
                        else {
                            0.0
                        };
                    let f = Spectrum::new(r, 1.0, 1.0);

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
