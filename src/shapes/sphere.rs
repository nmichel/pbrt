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

use super::{solid_angle_from_area, AreaSampleable, Shape, ShapeSample, SolidAngleSample};

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

    /// Draws a point of the sphere by drawing a direction **inside the cone it subtends**.
    ///
    /// Reference: PBR Book, 4ed, §6.2 — *Spheres*, « Sampling ».
    /// <https://pbr-book.org/4ed/Shapes/Spheres#Sampling>
    ///
    /// # What this is for
    ///
    /// Drawing over the area spends half its samples or more on the side facing away, which a
    /// one-sided emitter then throws out — `(1 − r/d)/2` of them survive, two in three on the
    /// witness scene. Every direction inside the cone, on the other hand, meets the sphere, so
    /// **no draw is wasted**; and the density is constant over that cone, where the converted
    /// area density varies across the visible cap by the whole span of `d²/cos θₗ`.
    ///
    /// # Derivation
    ///
    /// Write `d` for the distance from `reference` to the centre. The sphere is seen inside a cone
    /// of half-angle θmax about the axis towards the centre, the extreme direction being the one
    /// grazing the surface, where the radius meets the line of sight at a right angle:
    ///
    /// ```text
    /// [1]  sin θmax = r / d
    /// [2]  Ω = 2π·(1 − cos θmax)                      solid angle of a cone of half-angle θmax
    /// ```
    ///
    /// Uniform over that solid angle is uniform in φ **and in cos θ**, by the same argument that
    /// makes [`sample_area`](Self::sample_area) uniform in cos θ:
    ///
    /// ```text
    /// [3]  cos θ = 1 − u.x·(1 − cos θmax)             so cos θ ∈ [cos θmax, 1]
    /// [4]  φ     = 2π·u.y
    /// [5]  p(ω)  = 1/Ω                                constant over the cone
    /// ```
    ///
    /// A direction is not yet a point, and the point is what an emitter is read at. Put the
    /// reference at the origin of a frame whose z points at the centre, so the centre sits at
    /// (0, 0, d). A direction at angle θ from z meets the sphere where `‖t·ω − c‖ = r`, whose
    /// nearer root is
    ///
    /// ```text
    /// [6]  t = d·cos θ − √(r² − d²·sin²θ)
    /// ```
    ///
    /// Let α be the angle **at the centre** between the direction back to the reference and the
    /// direction to the point drawn. Projecting onto the axis,
    ///
    /// ```text
    /// [7]  cos α = (d − t·cos θ) / r
    ///            = sin²θ/sin θmax + cos θ·√(1 − sin²θ/sin²θmax)        [6] and [1]
    /// ```
    ///
    /// and the outward normal is that direction, which with [4] names the point entirely:
    ///
    /// ```text
    /// [8]  n = (sin α cos φ, sin α sin φ, −cos α)     in the frame above
    /// [9]  p = centre + r·n
    /// ```
    ///
    /// **Only the axial component is negated**, and the sign of the transverse one is not a detail
    /// to be taken on trust: α is measured at the centre from the direction pointing *back* at the
    /// reference, which is −z here, so the normal leans away from the axis on the **same** side as
    /// the direction drawn. Negating the whole vector instead puts the point at azimuth φ+π while
    /// the direction sits at φ — the two then name opposite sides of the sphere, and a shadow ray
    /// aimed at the point travels somewhere the density knows nothing about. At α = 0 both forms
    /// agree, which is why the face-on case cannot tell them apart.
    ///
    /// # Two cancellations removed, where pbrt expands a series
    ///
    /// `1 − cos θmax` of [2] is a subtraction of nearly equal numbers when the sphere is small or
    /// far, and so is `sin²θ = 1 − cos²θ` of [3] for the same reason. pbrt guards both with a
    /// Taylor expansion below `sin²θmax < sin²(1.5°)`. An exact identity does it with no threshold
    /// and no approximation at all:
    ///
    /// ```text
    /// [10]  1 − cos θmax = sin²θmax / (1 + cos θmax)          (1 − c)(1 + c) = 1 − c² = s²
    /// [11]  sin²θ = (1 − cos θ)·(1 + cos θ)                   with 1 − cos θ = u.x·(1 − cos θmax)
    /// ```
    ///
    /// Both right-hand sides are built from sums of positive quantities, so nothing cancels for any
    /// configuration — see `docs/arithmetique_flottante.md` for why that is the form to prefer. The
    /// departure from the reference is therefore towards more accuracy, not less.
    ///
    /// # The direction comes from the draw, never from the point
    ///
    /// ```text
    /// [12]  ω = (sin θ cos φ, sin θ sin φ, cos θ)     in the frame above
    /// ```
    ///
    /// θ and φ *are* what was drawn; the point is what [7] derives from them. Recovering the
    /// direction from the point instead — normalising `p − reference` — is a round trip through a
    /// subtraction, and that subtraction cancels to nothing in a case that is not rare at all.
    ///
    /// **A reference point sitting on this very surface** is the case, and a path puts one there
    /// every time it lands on the lamp before running next event estimation from it. Carried into
    /// the shape's space, such a point lands at `r` give or take a few ulps. A hair *outside*, and
    /// the test above sends it here with `sin θmax` one ulp below 1 — whereupon [7] gives
    /// `cos α = 1` for every draw, the point collapses onto the reference, and the subtraction is
    /// `0/0`. Measured before [12] replaced it: one sample in two hundred came back `NaN`, and a
    /// pixel that met one was black for good, half the image at 512 paths per pixel.
    ///
    /// [12] cannot do that: it is a unit vector built from an angle, whatever the geometry. The
    /// degenerate sample is still a poor one — it reports the surface the shaded point is already
    /// on — but a one-sided emitter answers nothing from it, which is the correct answer for a
    /// convex source: every other point of a sphere lies behind the tangent plane at any point of
    /// it.
    ///
    /// # Inside the sphere
    ///
    /// There is no cone: every direction meets the surface, and the draw falls back to the area
    /// one. An emissive sphere lights nothing inside itself in any case — the inner face points
    /// away — but the fallback is what keeps the answer defined rather than a special case that
    /// has to be remembered.
    fn sample_solid_angle(&self, reference: &Vector3f, u: &Vector2f) -> Option<SolidAngleSample> {
        // The sphere is centred on the origin of its own space, so the reference point's distance
        // to the centre is its own length.
        let squared_distance = vector3::dot(reference, reference);
        let squared_radius = self.r * self.r;

        if squared_distance <= squared_radius {
            return solid_angle_from_area(self.sample_area(u), reference);
        }

        let sin2_theta_max = squared_radius / squared_distance; // (1), squared
        let cos_theta_max = (1.0 - sin2_theta_max).max(0.0).sqrt();
        let one_minus_cos_theta_max = sin2_theta_max / (1.0 + cos_theta_max); // (10)

        let one_minus_cos_theta = u.x * one_minus_cos_theta_max; // (3)
        let cos_theta = 1.0 - one_minus_cos_theta;
        let sin2_theta = one_minus_cos_theta * (1.0 + cos_theta); // (11)

        // (7). The argument of the root is 1 − sin²θ/sin²θmax, which [3] keeps in [0, 1]; it is
        // clamped only against the rounding that can push it a hair below zero at θ = θmax.
        let cos_alpha = sin2_theta / sin2_theta_max.sqrt() + cos_theta * (1.0 - sin2_theta / sin2_theta_max).max(0.0).sqrt();
        let sin_alpha = (1.0 - cos_alpha * cos_alpha).max(0.0).sqrt();

        let phi = 2.0 * PI * u.y; // (4)
        let (cos_phi, sin_phi) = (phi.cos(), phi.sin());

        // The frame of the derivation: z towards the centre, the other two axes arbitrary — which
        // is all φ being uniform asks of them.
        let axis = (reference * -1.0).normalized();
        let (tangent, bitangent) = vector3::coordinate_system(&axis);

        // (8)
        let n = &tangent * (sin_alpha * cos_phi) + &bitangent * (sin_alpha * sin_phi) - &axis * cos_alpha;
        let p = &n * self.r; // (9)

        // (12). Never `(p − reference).normalized()`: θ and φ *are* the draw, and the point is
        // what was derived from them, so taking the direction back out of the point is a round
        // trip through a subtraction that can cancel to nothing. It does, and not rarely — see the
        // note on a reference sitting on the surface.
        let sin_theta = sin2_theta.sqrt();
        let wi = &tangent * (sin_theta * cos_phi) + &bitangent * (sin_theta * sin_phi) + &axis * cos_theta;

        let (u_tex, v_tex) = self.uv_at(&p);

        Some(SolidAngleSample {
            sp: SurfacePoint { p, n, u: u_tex, v: v_tex },
            wi,
            pdf: 1.0 / (2.0 * PI * one_minus_cos_theta_max), // (5) with (2)
        })
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
    /// The surface parameters of a point of the sphere, from the point alone.
    ///
    /// `u = φ/2π` and `v = θ/π`, the mapping the module's derivation states. It lives here rather
    /// than at its two call sites because a point *drawn* and a point *hit* have to name the same
    /// place: two copies of this would be two chances for a textured lamp to light a scene with one
    /// colour and be seen with another, and nothing in an image would say which was wrong.
    ///
    /// [`sample_area`](<Self as AreaSampleable>::sample_area) is the exception that does not call
    /// it, and deliberately: it *builds* the point from φ and θ, so it holds both exactly and would
    /// only lose a rounding by recovering them.
    fn uv_at(&self, p: &Vector3f) -> (f64, f64) {
        let mut phi = p.y.atan2(p.x);
        if phi < 0.0 {
            phi += 2.0 * PI;
        }

        let theta = clamp(p.z / self.r, -1.0, 1.0).acos();

        (phi / (2.0 * PI), theta / PI)
    }

    fn compute_intersection_details(&self, ray: &Ray, t: f64) -> Intersection {
        let hit = &ray.origin + &(&ray.direction * t);
        let mut norm = hit;
        norm.normalize();

        let (u, v) = self.uv_at(&hit);
        let theta = v * PI;

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
    /// Somewhere off every axis, so a frame built around the direction to the centre is never the
    /// shape's own and a permuted coordinate cannot hide.
    fn reference() -> Vector3f {
        Vector3f::new(3.0, -2.0, 4.0)
    }

    /// The half-angle of the cone the sphere subtends from [`reference`], and its solid angle.
    fn cone() -> (f64, f64) {
        let distance = reference().length();
        let cos_theta_max = (1.0 - RADIUS * RADIUS / (distance * distance)).sqrt();

        (cos_theta_max, 2.0 * PI * (1.0 - cos_theta_max))
    }

    /// `Σ 1/pdf / N` must converge to the solid angle the sphere subtends, `2π(1 − cos θmax)`.
    ///
    /// The counterpart, one measure up, of the area conservation test above, and it catches the
    /// same class of error: a density off by a constant factor, which scales how much light a
    /// source gives without changing anything one can see about it. Here the density is constant
    /// over the cone, so every single draw must already *be* the answer — the sum only says so
    /// loudly.
    #[test]
    fn test_the_density_integrates_to_the_solid_angle() {
        let sphere = Sphere::new(RADIUS);
        let mut sampler = sampler();
        let (_, solid_angle) = cone();

        let mut total = 0.0;
        for _ in 0..SAMPLES {
            total += 1.0 / sphere.sample_solid_angle(&reference(), &sampler.get_2d()).unwrap().pdf;
        }

        let estimate = total / SAMPLES as f64;
        assert!(
            (estimate / solid_angle - 1.0).abs() < TOLERANCE,
            "solid angle estimated at {} against an exact {}",
            estimate,
            solid_angle
        );
    }

    /// The draw is uniform **over the cone**, which a constant density alone does not say.
    ///
    /// `Σ f(ω)/pdf / N` estimates `∫ f dω` over the cone, and the first moment of the cosine is the
    /// integral a crowded draw cannot match:
    ///
    /// ```text
    /// ∫ cos θ dω = 2π ∫[cos θmax, 1] c dc = π·(1 − cos²θmax) = π·sin²θmax
    /// ```
    ///
    /// Drawing θ uniformly over [0, θmax] rather than its cosine is the mistake this sees — the
    /// same one the area draw guards against, made again a measure higher, and just as invisible to
    /// a test that only adds densities up.
    #[test]
    fn test_the_draw_is_uniform_over_the_cone() {
        let sphere = Sphere::new(RADIUS);
        let mut sampler = sampler();
        let (cos_theta_max, _) = cone();
        let axis = (reference() * -1.0).normalized();

        let mut total = 0.0;
        for _ in 0..SAMPLES {
            let sample = sphere.sample_solid_angle(&reference(), &sampler.get_2d()).unwrap();
            total += vector3::dot(&sample.wi, &axis) / sample.pdf;
        }

        let estimate = total / SAMPLES as f64;
        let exact = PI * (1.0 - cos_theta_max * cos_theta_max);
        assert!(
            (estimate / exact - 1.0).abs() < TOLERANCE,
            "∫cos θ dω estimated at {} against an exact {}",
            estimate,
            exact
        );
    }

    /// **Not one draw is wasted**, which is the whole point of sampling the cone.
    ///
    /// Every point comes back on the surface and facing the reference. The second half is what the
    /// area draw cannot say: there, the far side is drawn as often as the near one and a one-sided
    /// emitter throws all of it away. Here the near root of the ray is taken by construction, so a
    /// back-facing point would mean the derivation picked the wrong one — and the light that reads
    /// this would quietly drop a share of its samples again.
    #[test]
    fn test_every_drawn_point_faces_the_reference() {
        let sphere = Sphere::new(RADIUS);
        let mut sampler = sampler();

        for _ in 0..10_000 {
            let sample = sphere.sample_solid_angle(&reference(), &sampler.get_2d()).unwrap();

            assert!((sample.sp.p.length() - RADIUS).abs() < 1e-9, "{} is not on the sphere", sample.sp.p);

            // `wi` runs from the reference towards the point, so the direction back is its opposite.
            let cos_theta_l = vector3::dot(&sample.sp.n, &(sample.wi * -1.0));
            assert!(cos_theta_l > 0.0, "a point was drawn on the far side, cos θₗ = {}", cos_theta_l);
        }
    }

    /// The two draws measure the same thing, which is what says the new one is unbiased.
    ///
    /// Both estimate `∫ dω` over the directions that meet the **visible** cap, and must agree on
    /// its value — the cone draw exactly, every sample of it being the answer, and the area draw
    /// noisily, since it spends most of its samples on the far side where the front-facing test
    /// discards them.
    ///
    /// This is the tie between the two methods, and the reason it is worth a test of its own:
    /// nothing else would catch a cone draw that is internally consistent — a density that
    /// integrates to its own wrong solid angle — and disagrees with the conversion that was there
    /// first.
    #[test]
    fn test_the_two_draws_measure_the_same_solid_angle() {
        let sphere = Sphere::new(RADIUS);
        let mut sampler = sampler();
        let (_, solid_angle) = cone();

        let mut total = 0.0;
        for _ in 0..SAMPLES {
            let sample = solid_angle_from_area(sphere.sample_area(&sampler.get_2d()), &reference());
            if let Some(sample) = sample {
                // Only the cap facing the reference projects onto the cone; the far one covers the
                // same directions a second time and would count every one of them twice.
                if vector3::dot(&sample.sp.n, &(sample.wi * -1.0)) > 0.0 {
                    total += 1.0 / sample.pdf;
                }
            }
        }

        let estimate = total / SAMPLES as f64;
        assert!(
            (estimate / solid_angle - 1.0).abs() < TOLERANCE,
            "the area draw measures {} where the cone draw measures {}",
            estimate,
            solid_angle
        );
    }

    /// A reference point inside the sphere has no cone, and falls back to the area draw.
    ///
    /// There is no silhouette to subtend from in there, so `sin θmax = r/d` passes one and the
    /// derivation stops meaning anything. The fallback keeps an answer defined where a guard
    /// returning `None` would leave a caller to wonder which of its own assumptions had failed.
    #[test]
    fn test_a_reference_inside_the_sphere_falls_back_to_the_area_draw() {
        let sphere = Sphere::new(RADIUS);
        let mut sampler = sampler();
        let inside = Vector3f::new(0.3, -0.2, 0.1);

        for _ in 0..1000 {
            let sample = sphere.sample_solid_angle(&inside, &sampler.get_2d()).unwrap();
            assert!((sample.sp.p.length() - RADIUS).abs() < 1e-9, "{} is not on the sphere", sample.sp.p);
            assert!(sample.pdf.is_finite() && sample.pdf > 0.0, "pdf is {}", sample.pdf);
        }
    }

    /// A point drawn **in the cone** is the point a ray finds there, named the same way.
    ///
    /// The same property as the test below, for the other draw — and it needs its own because the
    /// two build their point by entirely different routes: one from the sphere's own angles, this
    /// one from an angle at the reference turned into an angle at the centre. The (u, v) are the
    /// half that fails in silence, and here they come from `uv_at`, so what is really under test is
    /// that the point itself landed where the derivation says.
    #[test]
    fn test_a_point_drawn_in_the_cone_is_the_point_a_ray_finds_there() {
        let sphere = Sphere::new(RADIUS);
        let mut sampler = sampler();

        for _ in 0..1000 {
            let sample = sphere.sample_solid_angle(&reference(), &sampler.get_2d()).unwrap();

            // Along the very direction the draw reports, from the reference point itself.
            let ray = Ray::new(&reference(), &sample.wi);
            let hits = sphere.intersect(&ray, 0.0, 100.0);

            assert!(!hits.is_empty(), "the direction towards {} must meet the sphere", sample.sp.p);
            let hit = hits[0];

            assert!((hit.p - sample.sp.p).length() < 1e-9, "hit at {}, drawn at {}", hit.p, sample.sp.p);
            assert!((hit.u - sample.sp.u).abs() < 1e-9, "u: hit {}, drawn {}", hit.u, sample.sp.u);
            assert!((hit.v - sample.sp.v).abs() < 1e-9, "v: hit {}, drawn {}", hit.v, sample.sp.v);
        }
    }

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
