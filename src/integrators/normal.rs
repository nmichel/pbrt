use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::scene::Scene;
use crate::spectrum::Spectrum;

use super::Integrator;

pub struct NormalIntegrator {}

impl NormalIntegrator {
    pub fn new() -> Self {
        Self {}
    }

    fn background_radiance(&self, _ray: &Ray, _scene: &Scene) -> Spectrum {
        Spectrum::new(0.0, 0.0, 0.0)
    }
}

impl Integrator for NormalIntegrator {
    fn li(&self, ray: &Ray, scene: &Scene, _depth: usize, near: f64, far: f64) -> Spectrum {
        match scene.intersect(&ray, near, far) {
            Some(interaction) => {
                let Interaction { ref intersection, .. } = interaction;
                let mut normal = intersection.n;
                normal = normal + Vector3f::new(1.0, 1.0, 1.0);
                let Vector3f { x, y, z } = normal * 0.5;
                Spectrum::new(x, y, z)
            }
            None => self.background_radiance(&ray, &scene),
        }
    }
}
