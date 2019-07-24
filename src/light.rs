use super::geom::ray::Ray;
use super::geom::vector3::Vector3f;
use super::spectrum::Spectrum;

pub trait Light {
    fn le(&self, _ray: &Ray) -> Spectrum {
        Spectrum::new(0.0, 0.0, 0.0)
    }
}

pub struct PointLight {
    pub p: Vector3f
}

impl PointLight {
    pub fn new(p: Vector3f) -> Self {
        PointLight { p }
    }
}

impl Light for PointLight {

}
