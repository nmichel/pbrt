use crate::geom::ray::Ray;
use crate::interaction::Interaction;
use crate::scene::Scene;
use crate::spectrum::Spectrum;

use super::integrator::Integrator;

pub struct PathIntegrator {
    /// Max recursion depth
    max_depth: usize,
}

impl PathIntegrator {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }

    fn background_radiance(&self, ray: &Ray, _scene: &Scene) -> Spectrum {
        let mut unit_direction = ray.direction;
        unit_direction.normalize();
        let t = 0.5*(unit_direction.y + 1.0);
        return Spectrum::new(1.0, 1.0, 1.0)*(1.0-t) + Spectrum::new(0.5, 0.7, 1.0) * t;
    }
}

impl Integrator for PathIntegrator {
    fn li(&self, ray: &Ray, scene: &Scene, depth: usize, near: f64, far: f64) -> Spectrum {
        if depth <= 0 {
            return Spectrum::BLACK;
        }
        match scene.intersect(&ray, near, far) {
            Some(interaction) => {
                let Interaction { material, .. } = interaction;
                let emitted = match material.emit(&ray, &interaction) {
                    Some(emitted) => emitted,
                    None => Spectrum::new(0.0, 0.0, 0.0),
                };
                match material.scatter(ray, &interaction) {
                    Some((attenuation, scattered)) => {
                        emitted
                            + attenuation * &self.li(&scattered, scene, depth - 1, near, far)
                    }
                    None => emitted,
                }
            }
            None => self.background_radiance(&ray, &scene),
        }
    }
}
