use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::interaction::Interaction;
use crate::materials::Material;
use crate::scene::Scene;
use crate::spectrum::Spectrum;

use super::integrator::Integrator;

pub struct NormalIntegrator {
}

impl NormalIntegrator {
    pub fn new() -> Self {
        Self { }
    }

    fn background_radiance(&self, _ray: &Ray, _scene: &Scene) -> Spectrum {
        Spectrum::new(0.0, 0.0, 0.0)
    }
}

impl Integrator for NormalIntegrator {
    fn li(&self, ray: &Ray, scene: &Scene, depth: usize, near: f64, far: f64) -> Spectrum {
        match scene.intersect(&ray, near, far) {
            Some(interaction) => {
                let Interaction { ref intersection, .. } = interaction;
                Spectrum::new(intersection.n.x.abs(), intersection.n.y.abs(), intersection.n.z.abs())
            },
            None => {
                self.background_radiance(&ray, &scene)
            }
        }
    }
}
