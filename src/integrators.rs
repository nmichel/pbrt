use crate::colors;
use crate::geom::ray::Ray;
use crate::scene::Scene;
use crate::spectrum::Spectrum;

pub trait Integrator: Send + Sync {
    fn li(&self, ray: &Ray, scene: &Scene, depth: usize, near: f64, far: f64) -> Spectrum;

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
