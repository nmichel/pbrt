use crate::geom::vector2::Vector2f;
use crate::geom::vector3::Vector3f;
use std::f64::consts::PI;

use super::Pdf;

/// Directions of the upper hemisphere, drawn with a density proportional to cos θ.
///
/// Reference: PBR Book, 4ed, §A.5.3 — *Cosine-Weighted Hemisphere Sampling*.
/// <https://pbr-book.org/4ed/Sampling_Algorithms/Sampling_Multidimensional_Functions#Cosine-WeightedHemisphereSampling>
///
/// # Frame
///
/// The shading frame, where `z` is the normal: θ is the angle to `+z`, φ the azimuth around it,
/// and `ω = (sin θ cos φ, sin θ sin φ, cos θ)`. The hemisphere is `z > 0`.
///
/// # Why a cosine
///
/// What a diffuse surface integrates is `f · Lᵢ · cos θ`, with `f = ρ/π` constant. A density
/// proportional to cos θ cancels the only factor of that integrand which is known in advance:
///
/// ```text
///   f · cos θ / p(ω)  =  (ρ/π) · cos θ / (c · cos θ)  =  ρ / (π c)
/// ```
///
/// a constant, so whatever noise is left comes from `Lᵢ` alone.
///
/// # The constant c
///
/// A density integrates to 1 over its domain. With the solid angle element of spherical
/// coordinates, `dω = sin θ dθ dφ`:
///
/// ```text
///   ∫_H² c cos θ dω  =  c ∫₀^2π ∫₀^π/2 cos θ sin θ dθ dφ
///                    =  c · 2π · [ sin²θ / 2 ]₀^π/2                    [sin θ cos θ = (sin²θ/2)′]
///                    =  c · 2π · 1/2  =  c π  =  1
/// ```
///
/// hence `c = 1/π`, and
///
/// ```text
///   [1]  p(ω) = cos θ / π
/// ```
///
/// which also makes the weight above exactly ρ: a lambertian sample loses nothing to its density.
///
/// # How `generate` produces it: Malley's method
///
/// Draw a point uniformly on the unit disk, and lift it vertically onto the hemisphere.
///
/// - **Uniform on the disk.** In polar coordinates `(r, φ)` the area element is `r dr dφ`, so a
///   density uniform in area, `1/π`, is in `(r, φ)`:
///
///   ```text
///   [2]  p(r, φ) = r / π                                              [jacobian of polar coordinates]
///   ```
///
///   Its marginal in `r` is `2r`, whose cumulative distribution is `r²`. Inverting it gives
///   `r = √u₂`, and φ is uniform, `φ = 2π u₁`.
///
/// - **Lifted onto the hemisphere.** The point at radius `r` goes to the direction whose projection
///   on the disk it is, so `r = sin θ`, and `z = cos θ = √(1 − r²)`. Changing variable from `r` to θ:
///
///   ```text
///   [3]  p(θ, φ) = p(r, φ) · |dr/dθ| = (sin θ / π) · cos θ             [2], [change of variable]
///   ```
///
/// - **Back to solid angle.** A density over `(θ, φ)` and one over directions differ by the
///   `sin θ` of `dω = sin θ dθ dφ`:
///
///   ```text
///   p(ω) = p(θ, φ) / sin θ = cos θ / π                                [3]
///   ```
///
///   which is `[1]`: the lifted disk is the cosine-weighted hemisphere.
///
/// # What `value` answers below the horizon
///
/// `generate` never produces `z < 0`, and `[1]` holds on the upper hemisphere only. `value` returns
/// `|cos θ| / π` there rather than 0, which is the density of the mirrored distribution; a caller
/// that needs the honest 0 — a bsdf asked about a direction on the other side — tests the
/// hemisphere itself.
pub struct CosinePdf {}

impl Pdf for CosinePdf {
    fn value(&self, direction: &Vector3f) -> f64 {
        (direction.z / PI).abs()
    }

    fn generate(&self, u: &Vector2f) -> Vector3f {
        let two_pi = 2.0 * PI;
        let phi = two_pi * u.x;
        let r2 = u.y;
        let r2_sqrt = r2.sqrt();
        let cos_theta = (1.0 - r2).sqrt();
        let x = phi.cos() * r2_sqrt;
        let y = phi.sin() * r2_sqrt;
        let z = cos_theta;
        Vector3f::new(x, y, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::vector2::Vector2u;
    use crate::geom::vector3::Vector3f;
    use crate::pdfs::cosine::CosinePdf;
    use crate::samplers::{IndependentSampler, Sampler};
    use std::f64::consts::FRAC_1_PI;

    #[test]
    fn test_lambertian_energy_conservation_cosine_sampling() {
        let mut total = 0.0;
        let samples = 100000;
        let pdf = CosinePdf {};

        // A fixed seed, so this test either passes or fails, always the same way.
        let mut sampler = IndependentSampler::new(0, &Vector2u::new(0, 0), 0);

        for _ in 0..samples {
            // Sample a direction in local coordinates (z+ is normal)
            let wi: Vector3f = pdf.generate(&sampler.get_2d());
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
