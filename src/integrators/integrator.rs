use crate::geom::ray::Ray;
use crate::scene::Scene;
use crate::spectrum::Spectrum;

pub trait Integrator : Send + Sync {
    fn li(&self, ray: &Ray, scene: &Scene, depth: usize, near: f64, far: f64) -> Spectrum;
}
