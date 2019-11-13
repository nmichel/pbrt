use crate::geom::ray::Ray;
use crate::scene::Scene;
use crate::spectrum::Spectrum;

pub trait Integrator {
    fn li(&self, ray: &Ray, scene: &Scene, depth: usize) -> Spectrum;
}
