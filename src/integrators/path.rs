use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::interaction::Interaction;
use crate::materials::material::Material;
use crate::scene::Scene;
use crate::spectrum::Spectrum;

use super::integrator::Integrator;

pub struct PathIntegrator {
    /// Max recursion depth
    max_depth: usize
}

impl PathIntegrator {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }
}

impl Integrator for PathIntegrator {
    fn li(&self, ray: &Ray, scene: &Scene, depth: usize, near: f64, far: f64) -> Spectrum {
        match scene.intersect(&ray, near, far) {
            Some(interaction) => {
                let Interaction { ref material, .. } = interaction;
                if depth > 0 {
                    match material.scatter(ray, &interaction) {
                        Some((attenuation, scattered)) => 
                            attenuation * &self.li(&scattered, scene, depth-1, near, far),
                        None =>
                           Spectrum::new(0.0, 0.0, 0.0)
                    }
                }
                else {
                    Spectrum::new(0.0, 0.0, 0.0)
                }
            },
            None => {
                scene.background_radiance(&ray)
            }
        }
    }
}
