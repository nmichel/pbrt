use crate::geom::vector3::Vector3f;

pub trait Pdf {
    fn value(&self, _direction: &Vector3f) -> f64;

    fn generate(&self) -> Vector3f;
}

pub mod cosine;
pub mod hemisphere;
pub mod sphere;
