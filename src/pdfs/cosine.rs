use crate::geom::vector3::Vector3f;
use crate::utils::random_double;
use std::f64::consts::PI;

use super::Pdf;

pub struct CosinePdf {}

impl Pdf for CosinePdf {
    fn value(&self, direction: &Vector3f) -> f64 {
        (direction.z / PI).max(0.0)
    }

    fn generate(&self) -> Vector3f {
        let two_pi = 2.0 * PI;
        let phi = two_pi * random_double();
        let r2 = random_double();
        let r2_sqrt = r2.sqrt();
        let cos_theta = (1.0 - r2).sqrt();
        let x = phi.cos() * r2_sqrt;
        let y = phi.sin() * r2_sqrt;
        let z = cos_theta;
        Vector3f::new(x, y, z)
    }
}
