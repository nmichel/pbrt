use crate::geom::vector2::Vector2f;
use crate::geom::vector3::Vector3f;
use std::f64::consts::PI;

use super::Pdf;

pub struct SpherePdf {}

impl Pdf for SpherePdf {
    fn value(&self, _direction: &Vector3f) -> f64 {
        1.0 / (4.0 * PI)
    }

    fn generate(&self, u: &Vector2f) -> Vector3f {
        let two_pi = 2.0 * PI;
        let phi = two_pi * u.x;
        let cos_theta = 1.0 - 2.0 * u.y;
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
        let x = phi.cos() * sin_theta;
        let y = phi.sin() * sin_theta;
        let z = cos_theta;
        Vector3f::new(x, y, z)
    }
}
