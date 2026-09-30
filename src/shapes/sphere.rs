use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, Intersection, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::surface_point::SurfacePoint;
use crate::geom::vector2::Vector2f;
use crate::geom::vector3;
use crate::geom::vector3::Vector3f;
use num_traits::clamp;
use std::f64::consts::PI;
use std::sync::Arc;

use super::{AreaSampleable, Shape, ShapeSample};

pub struct Sphere {
    r: f64,
}

impl Sphere {
    pub fn new(r: f64) -> Sphere {
        Sphere { r }
    }

    pub fn radius(&self) -> f64 {
        self.r
    }
}

impl Shape for Sphere {
    fn area_sampler(self: Arc<Self>) -> Option<Arc<dyn AreaSampleable>> {
        Some(self)
    }
}

impl AreaSampleable for Sphere {
    /// The area of a sphere of radius r.
    fn area(&self) -> f64 {
        4.0 * PI * self.r * self.r
    }

    /// A point drawn uniformly over the surface.
    ///
    /// Reference: PBR Book, 4ed, §A.5.2 — *Uniformly Sampling Hemispheres and Spheres*.
    /// <https://pbr-book.org/4ed/Sampling_Algorithms/Sampling_Multidimensional_Functions#UniformlySamplingHemispheresandSpheres>
    ///
    /// # Derivation
    ///
    /// In the spherical coordinates `intersect` uses above — polar axis z, φ measured in the
    /// (x, y) plane — the element of area is
    ///
    /// ```text
    /// [1]  dA = r² sin θ dθ dφ
    /// ```
    ///
    /// which is *not* uniform in (θ, φ): the same step in θ sweeps less surface near a pole than at
    /// the equator. Substituting `c = cos θ`, whose differential is `dc = −sin θ dθ`, removes the
    /// very factor that makes it so:
    ///
    /// ```text
    /// [2]  dA = −r² dc dφ                                         [1] with c = cos θ
    /// ```
    ///
    /// So area is uniform in **(φ, cos θ)**, and a `u` uniform over the unit square gives a point
    /// uniform over the surface through
    ///
    /// ```text
    /// [3]  φ     = 2π·u.x
    /// [4]  cos θ = 1 − 2·u.y                                      so cos θ ∈ (−1, 1]
    /// [5]  p     = r·(sin θ cos φ, sin θ sin φ, cos θ)            sin θ = √(1 − cos²θ) ≥ 0
    /// ```
    ///
    /// The jacobian of [3][4] is the constant `4πr²` of [2], which is [`area`](Self::area), so the
    /// density is the same everywhere:
    ///
    /// ```text
    /// [6]  p(A) = 1 / area = 1 / (4πr²)
    /// ```
    ///
    /// **Drawing θ uniformly instead of its cosine is the mistake to avoid**, and it is a quiet one:
    /// the points still cover the whole surface, so nothing is missing from the image — they simply
    /// crowd around the poles, and an emitter would then light a scene as though it were brighter
    /// at its top and bottom than around its middle.
    ///
    /// This is the same map [`SpherePdf`](crate::pdfs::sphere::SpherePdf) applies to draw a
    /// *direction*, and that is not a coincidence: a direction is a point of the unit sphere, so
    /// "uniform over the area of a sphere" and "uniform over directions" are one question asked
    /// twice.
    ///
    /// # The texture coordinates are the ones `intersect` reports
    ///
    /// `u = φ/2π` and `v = θ/π`, exactly as `compute_intersection_details` derives them from a hit
    /// point. A drawn point and a hit point must name the same place on the same surface, or a
    /// textured emitter lights a scene with one colour and is seen with another. Here [3] makes the
    /// first of them free: `u` *is* `u.x`.
    ///
    /// The normal points outwards, so a shaded point **inside** an emissive sphere receives
    /// nothing — emission being one-sided, `cos θₗ ≤ 0` answers "no sample" for every point drawn.
    /// That is the honest reading of a one-sided surface, not a special case worth guarding.
    fn sample_area(&self, u: &Vector2f) -> ShapeSample {
        let phi = 2.0 * PI * u.x; // (3)
        let cos_theta = 1.0 - 2.0 * u.y; // (4)
        let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();

        // (5), split from the normal so the radius is applied once and in one place.
        let n = Vector3f::new(sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta);

        ShapeSample {
            sp: SurfacePoint {
                p: &n * self.r,
                n,
                u: u.x,
                v: cos_theta.acos() / PI,
            },
            pdf: 1.0 / self.area(), // (6)
        }
    }
}

impl Intersectable for Sphere {
    /// See https://pbr-book.org/4ed/Shapes/Spheres
    /// see https://en.wikipedia.org/wiki/Spherical_coordinate_system
    /// See https://en.wikipedia.org/wiki/Chain_rule
    ///
    /// θ
    /// φ
    /// π
    /// δ
    ///
    /// 3D space to sperical coordinates
    /// ---
    ///
    /// φ = atan2(y, x)
    /// θ = acos(z/r)
    ///
    /// x = r sin(θ) cos(φ)   [1]
    /// y = r sin(θ) sin(φ)   [2]
    /// z = r cos(θ)
    ///
    /// Angles to u/v mapping
    /// ---
    ///
    /// φ in [0, 2π]
    /// θ in [0, π]
    /// u, v in [0, 1]
    ///
    /// φ(u) = 2π u
    /// θ(v) = π v
    ///
    /// Projection of intersection point / basic trigonometry
    /// ---
    ///
    /// cos(θ) = z/r   [3]
    /// cos(φ) = x/r   [4]
    /// sin(φ) = y/r   [5]
    ///
    /// Derivatives of (u, v) position (using Chain Rule)
    /// ---
    ///
    /// δx/δθ = δ(r sin(θ) cos(φ))/δθ
    ///       = r cos(θ) cos(φ)
    ///       = r (z/r) cos(φ)   [3]
    ///       = z cos(φ)
    ///
    /// δy/δθ = r cos(θ) sin(φ)
    ///       = r (z/r) sin(φ)   [3]
    ///       = z sin(φ)
    ///
    /// δz/δθ = -r sin(θ)
    ///
    /// δx/δφ = -r sin(θ) sin(φ)
    ///       = -y     [2]
    ///
    /// δy/δφ = r sin(θ) cos(φ)
    ///       = x       [1]
    ///
    /// δz/δφ = 0
    ///
    /// θ(v) = π v
    /// δθ/δv = π
    /// δx/δv = δx/δθ δθ/δv                           [Chain Rule]
    ///       = (r cos(θ) cos(φ)) δθ/δv
    ///       = π z cos(φ)
    /// δy/δv = δy/δθ δθ/δv
    ///       = π z sin(φ)
    /// δz/δv = -r π sin(θ)
    ///
    /// φ(u) = 2π u
    /// δφ/δu = 2π
    /// δx/δu = δx/δφ δφ/δu                           [Chain Rule]
    ///       = (-r sin(θ) sin(φ)) δφ/δu
    ///       = -2π y
    /// δy/δu = δy/δφ δφ/δu
    ///       = 2π x
    /// δz/δu = 0
    ///
    /// δp/δu = 2π(-y, x, 0)
    /// δp/δv = π(z cos(φ), z sin(φ), -r sin(θ))
    ///       = π(zx/r, zy/r, -r sin(θ))   [4][5]
    ///
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        // Compute intersection point (geometric solution):
        // https://www.scratchapixel.com/lessons/3d-basic-rendering/minimal-ray-tracer-rendering-simple-shapes/ray-sphere-intersection

        let l = &ray.origin * -1.0;
        let tca = vector3::dot(&l, &ray.direction);
        if tca < 0.0 {
            return IntersectionResult::new();
        }

        let r2 = self.r * self.r;
        let d2 = vector3::dot(&l, &l) - tca * tca;
        if d2 > r2 {
            return IntersectionResult::new();
        }

        let thc = (r2 - d2).sqrt();
        let t0: f64 = tca - thc;
        let t1: f64 = tca + thc;
        let tmin = f64::min(t0, t1);
        let tmax = f64::max(t0, t1);
        if tmax < near || tmin > far {
            return IntersectionResult::new();
        }

        let mut res = IntersectionResult::new();

        if tmin > near {
            res.push(self.compute_intersection_details(ray, tmin))
        }

        if tmax > near && tmax < far {
            res.push(self.compute_intersection_details(ray, tmax))
        }

        res
    }

    fn contain_point(&self, point: &Vector3f) -> bool {
        vector3::dot(&point, &point) <= self.r * self.r
    }
}

impl Sphere {
    fn compute_intersection_details(&self, ray: &Ray, t: f64) -> Intersection {
        let hit = &ray.origin + &(&ray.direction * t);
        let mut norm = hit;
        norm.normalize();

        // Compute UV coords (<=> polar coords)

        let mut phi = hit.y.atan2(hit.x);
        if phi < 0.0 {
            phi += 2.0 * PI;
        }
        let u = phi / (2.0 * PI);

        let theta = clamp(hit.z / self.r, -1.0, 1.0).acos();
        let v = theta / PI;

        // Compute UV derivatives

        let z_radius = (hit.x * hit.x + hit.y * hit.y).sqrt();
        let inv_z_r = 1.0 / z_radius;
        let cos_phi = hit.x * inv_z_r;
        let sin_phi = hit.y * inv_z_r;
        let dpdu = vector3::Vector3::new(-hit.y, hit.x, 0.0) * (2.0 * PI);
        let dpdv = vector3::Vector3::new(hit.z * cos_phi, hit.z * sin_phi, -self.r * theta.sin()) * PI;

        Intersection {
            p: hit,
            d: t,
            n: norm,
            wo: &ray.direction * -1.0,
            u,
            v,
            dpdu,
            dpdv,
        }
    }
}

impl AABound for Sphere {
    fn get_bounding_box(&self) -> AABoundingBox {
        let bmin = Vector3f::new(-self.r, -self.r, -self.r);
        let bmax = Vector3f::new(self.r, self.r, self.r);
        AABoundingBox::new(&bmin, &bmax)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::geom::vector2::Vector2u;
    use crate::samplers::{IndependentSampler, Sampler};

    /// A radius that is neither one nor a half, so a factor of r or r² cannot pass unnoticed.
    const RADIUS: f64 = 1.7;

    /// The same budget and tolerance the rectangle's draw is checked with: at 100 000 draws the
    /// estimators below sit within a few tenths of a percent of their exact value, so a percent
    /// never fails by chance and still catches the errors that matter — which are factors.
    const SAMPLES: usize = 100_000;
    const TOLERANCE: f64 = 0.01;

    fn sampler() -> IndependentSampler {
        // A fixed seed, so this test either passes or fails, always the same way.
        IndependentSampler::new(0, &Vector2u::new(0, 0), 0)
    }

    /// The Monte-Carlo estimator of the area, `Σ 1/pdf / N`, must converge to [`Sphere::area`].
    ///
    /// The density is constant, so this says nothing about the draw being *uniform* — the test
    /// below does that. What it catches is the error uniformity cannot show: a density off by a
    /// constant factor from the area it is meant to be the reciprocal of. Dropping the 4 of `4πr²`
    /// is the obvious one, and on an image it would only make the lamp four times too bright,
    /// which looks exactly like a lamp four times too bright.
    #[test]
    fn test_the_density_integrates_to_the_area() {
        let sphere = Sphere::new(RADIUS);
        let mut sampler = sampler();

        let mut total = 0.0;
        for _ in 0..SAMPLES {
            total += 1.0 / sphere.sample_area(&sampler.get_2d()).pdf;
        }

        let estimate = total / SAMPLES as f64;
        let exact = 4.0 * PI * RADIUS * RADIUS;
        assert!(
            (estimate / exact - 1.0).abs() < TOLERANCE,
            "area estimated at {} against an exact {}",
            estimate,
            exact
        );
    }

    /// The draw is uniform over the surface, tested against integrals a crowded draw cannot satisfy.
    ///
    /// `Σ f(p)/pdf / N` estimates `∫ f dA` for any `f`, and `f = 1` degenerates into the test above.
    /// Two moments are needed here as they are for the rectangle, and for the same reasons.
    ///
    /// **The first says where the points are centred.** The sphere is centred on the origin, so each
    /// coordinate integrates to zero over it.
    ///
    /// **The second says how they spread, and it is the one that matters here.** By symmetry the
    /// three coordinates integrate alike, so
    ///
    /// ```text
    /// ∫ x² dA = ∫ y² dA = ∫ z² dA = (1/3)·∫ (x² + y² + z²) dA = (1/3)·r²·area
    /// ```
    ///
    /// every point of the surface being at distance r from the centre. This is what catches the
    /// mistake the derivation warns about: drawing θ uniformly rather than its cosine still covers
    /// the whole surface, but crowds the points at the poles, and `∫ z² dA` then estimates
    /// `r²·area/2` instead of `r²·area/3` — a difference of half, on a test that reads the same
    /// under any error of a constant factor.
    ///
    /// Taking all three axes rather than one is what catches a permutation of the coordinates,
    /// which the polar axis being z makes possible.
    #[test]
    fn test_the_draw_is_uniform_over_the_surface() {
        let sphere = Sphere::new(RADIUS);
        let mut sampler = sampler();
        let area = 4.0 * PI * RADIUS * RADIUS;

        let mut means = [0.0; 3];
        let mut seconds = [0.0; 3];
        for _ in 0..SAMPLES {
            let sample = sphere.sample_area(&sampler.get_2d());
            let p = [sample.sp.p.x, sample.sp.p.y, sample.sp.p.z];

            for axis in 0..3 {
                means[axis] += p[axis] / sample.pdf;
                seconds[axis] += p[axis] * p[axis] / sample.pdf;
            }
        }

        let n = SAMPLES as f64;
        let names = ["x", "y", "z"];

        // The exact value is zero, so the departure is measured against the radius rather than
        // against itself: a centroid off by a percent of the extent is what the assertion allows.
        for axis in 0..3 {
            let estimate = means[axis] / n / area;
            assert!(
                estimate.abs() < TOLERANCE * RADIUS,
                "the centroid sits at {} on {}, and the surface is centred on the origin",
                estimate,
                names[axis]
            );
        }

        let exact = area * RADIUS * RADIUS / 3.0;
        for axis in 0..3 {
            let estimate = seconds[axis] / n;
            assert!(
                (estimate / exact - 1.0).abs() < TOLERANCE,
                "∫{}² dA estimated at {} against an exact {}",
                names[axis],
                estimate,
                exact
            );
        }
    }

    /// A point *drawn* on the surface and a point *hit* by a ray are the same point, named the same
    /// way.
    ///
    /// This is the property an area light rests on, and the (u, v) half of it is the one that can go
    /// wrong in silence. Both halves of the sphere's parameterisation are written twice — once from
    /// an angle in `sample_area`, once from a hit point in `compute_intersection_details` — and
    /// nothing but this test holds the two together. Getting `v` upside down, `1 − θ/π` for `θ/π`,
    /// would leave every image unchanged except one lit by a textured sphere.
    ///
    /// A ray aimed at the surface from outside meets it **twice**, the near face and the far one;
    /// the near one is the point drawn, and `intersect` reports them in that order.
    #[test]
    fn test_a_drawn_point_is_the_point_a_ray_finds_there() {
        let sphere = Sphere::new(RADIUS);
        let mut sampler = sampler();

        for _ in 0..1000 {
            let sample = sphere.sample_area(&sampler.get_2d());

            // Straight down the outward normal, from far enough away that the ray has to travel.
            let origin = sample.sp.p + sample.sp.n * (2.0 * RADIUS);
            let ray = Ray::spawn_from_through(&origin, &sample.sp.p);
            let hits = sphere.intersect(&ray, 0.0, 100.0);

            assert_eq!(hits.len(), 2, "a ray aimed at {} must cross the surface twice", sample.sp.p);
            let hit = hits[0];

            assert!((hit.p - sample.sp.p).length() < 1e-9, "hit at {}, drawn at {}", hit.p, sample.sp.p);
            assert!((hit.n - sample.sp.n).length() < 1e-9, "normal: hit {}, drawn {}", hit.n, sample.sp.n);
            assert!((hit.u - sample.sp.u).abs() < 1e-9, "u: hit {}, drawn {}", hit.u, sample.sp.u);
            assert!((hit.v - sample.sp.v).abs() < 1e-9, "v: hit {}, drawn {}", hit.v, sample.sp.v);
        }
    }
}
