use crate::colors;
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::samplers::Sampler;
use crate::scene::Scene;
use crate::spectrum::Spectrum;

use super::Integrator;

pub struct NormalIntegrator {}

impl NormalIntegrator {
    pub fn new() -> Self {
        Self {}
    }
}

impl Integrator for NormalIntegrator {
    fn li(&self, ray: &Ray, scene: &Scene, _depth: usize, near: f64, far: f64, _sampler: &mut dyn Sampler) -> Spectrum {
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

    /// Black, where every other integrator reads the scene's infinite lights.
    ///
    /// This one draws normals, not light: it maps a direction onto a colour, and the sky of
    /// [`BackgroundInfiniteLight`](crate::lights::BackgroundInfiniteLight) is a blue that reads
    /// as a +z normal. A background that says nothing is what tells a reader where the geometry
    /// ends.
    fn background_radiance(&self, _ray: &Ray, _scene: &Scene) -> Spectrum {
        colors::BLACK
    }
}
