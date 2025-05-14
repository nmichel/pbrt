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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::vector3::Vector3f;
    use crate::pdfs::hemisphere::HemispherePdf;
    use std::f64::consts::FRAC_1_PI;

    #[test]
    fn test_lambertian_energy_conservation_uniform_hemisphere() {
        let samples = 100_000;
        let pdf = HemispherePdf {};
        let mut total = 0.0;

        for _ in 0..samples {
            // Sample a direction in local coordinates (z+ is normal)
            let wi: Vector3f = pdf.generate();
            let cos_theta = wi.z.max(0.0);

            // Lambertian BRDF is 1/π
            let brdf = FRAC_1_PI;
            let pdf_val = pdf.value(&wi);

            // Monte Carlo estimate: f(wi) * cosθ / pdf(wi)
            let contribution = brdf * cos_theta / pdf_val;
            total += contribution;
        }

        let average = total / samples as f64;
        assert!((average - 1.0).abs() < 0.01, "Energy conservation failed: {}", average);
    }
}
