use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::interaction::Interaction;
use crate::materials::Material;
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

    fn background_radiance(&self, _ray: &Ray, _scene: &Scene) -> Spectrum {
        Spectrum::new(0.0, 0.0, 0.0)
    }
}

impl Integrator for PathIntegrator {
    fn li(&self, ray: &Ray, scene: &Scene, depth: usize, near: f64, far: f64) -> Spectrum {
        match scene.intersect(&ray, near, far) {
            Some(interaction) => {
                let Interaction { ref material, .. } = interaction;
                let emitted = match material.emit(&ray, &interaction) {
                    Some(emitted) => emitted,
                    None => Spectrum::new(0.0, 0.0, 0.0)
                };
                if depth > 0 {
                    match material.scatter(ray, &interaction) {
                        Some((attenuation, scattered)) => emitted + attenuation * &self.li(&scattered, scene, depth-1, near, far),
                        None => emitted
                    }
                }
                else {
                    emitted
                }
            },
            None => {
                self.background_radiance(&ray, &scene)
            }
        }
    }
}
