use crate::colors;
use crate::geom::ray::Ray;
use crate::samplers::Sampler;
use crate::scene::Scene;
use crate::spectrum::Spectrum;

pub trait Integrator: Send + Sync {
    /// Radiance arriving along `ray`, estimated by drawing from `sampler`.
    ///
    /// The sampler is the integrator's only source of numbers, and it hands it on to the
    /// materials and lights it queries: a path is therefore a deterministic function of the
    /// stream it is given.
    fn li(&self, ray: &Ray, scene: &Scene, depth: usize, near: f64, far: f64, sampler: &mut dyn Sampler) -> Spectrum;

    fn background_radiance(&self, _ray: &Ray, _scene: &Scene) -> Spectrum {
        colors::BLACK
    }
}

pub mod naive;
pub mod normal;
pub mod path;
// pub mod whitted;

pub use self::naive::NaiveIntegrator;
pub use self::normal::NormalIntegrator;
pub use self::path::PathIntegrator;

#[derive(Debug)]
pub enum Type {
    PATH,
    NAIVE,
    NORMAL,
}
