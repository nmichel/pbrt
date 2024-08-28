use crate::geom::vector3::Vector3f;
use crate::utils::random_double;
use std::f64::consts::PI;

use super::Pdf;

pub struct HemispherePdf {}

impl Pdf for HemispherePdf {
    fn value(&self, _direction: &Vector3f) -> f64 {
        1.0 / (2.0 * PI)
    }

    fn generate(&self) -> Vector3f {
        let two_pi = 2.0 * PI;
        let phi = two_pi * random_double();
        let cos_theta = 1.0 - random_double();
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
        let x = phi.cos() * sin_theta;
        let y = phi.sin() * sin_theta;
        let z = cos_theta;
        Vector3f::new(x, y, z)
    }
}
