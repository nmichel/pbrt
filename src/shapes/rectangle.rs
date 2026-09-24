use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, Intersection, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::surface_point::SurfacePoint;
use crate::geom::vector2::Vector2f;
use crate::geom::vector3;
use crate::geom::vector3::Vector3f;

use super::{AreaSampleable, Shape, ShapeSample};

pub struct Rectangle {
    half_width: f64,
    half_height: f64,
}

impl Rectangle {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            half_width: width / 2.0,
            half_height: height / 2.0,
        }
    }
}

impl Shape for Rectangle {}

impl AreaSampleable for Rectangle {
    /// The rectangle spans [-half_width, half_width] × [-half_height, half_height] in the y = 0
    /// plane, so its area is the product of the two full sides.
    fn area(&self) -> f64 {
        (2.0 * self.half_width) * (2.0 * self.half_height)
    }

    /// A point drawn uniformly, by mapping the unit square onto the rectangle.
    ///
    /// # Derivation
    ///
    /// The map is affine and separable, one coordinate per component of `u`:
    ///
    /// ```text
    /// [1]  x = (2·u.x − 1)·half_width          u.x ∈ [0, 1) ↦ x ∈ [-half_width, half_width)
    /// [2]  z = (2·u.y − 1)·half_height
    /// ```
    ///
    /// Its jacobian is constant — ∂(x, z)/∂(u.x, u.y) = 4·half_width·half_height, which is
    /// [`area`](Self::area) — so a `u` uniform over the unit square gives a point uniform over the
    /// surface, and the density is the same everywhere:
    ///
    /// ```text
    /// [3]  p(x, z) = p(u) / |∂(x, z)/∂(u)| = 1 / area
    /// ```
    ///
    /// **The texture coordinates are the ones `intersect` reports**, `u = x` and `v = z`, and that
    /// is not a detail: they are what a texture is read at. Normalising them to [0, 1) here — which
    /// would look tidier — would make a drawn point and a hit point name different places on the
    /// same surface, so a textured emitter would light a scene with one colour and be seen with
    /// another.
    ///
    /// The normal is the geometric one, unturned: which face it points to is the orientation the
    /// scene chose, and a one-sided emitter reads it.
    fn sample_area(&self, u: &Vector2f) -> ShapeSample {
        let x = (2.0 * u.x - 1.0) * self.half_width;
        let z = (2.0 * u.y - 1.0) * self.half_height;

        ShapeSample {
            sp: SurfacePoint {
                p: Vector3f::new(x, 0.0, z),
                n: Vector3f::new(0.0, 1.0, 0.0),
                u: x,
                v: z,
            },
            pdf: 1.0 / self.area(),
        }
    }
}

impl Intersectable for Rectangle {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        let mut res = IntersectionResult::new();

        if ray.direction.y == 0.0 {
            res
        }
        else {
            let d = (ray.origin.y * -1.0) / ray.direction.y;
            if d < near || d > far {
                return res;
            }

            let mut p = ray.origin + ray.direction * d;
            p.y = 0.0;

            if p.x.abs() > self.half_width || p.z.abs() > self.half_height {
                return res;
            }

            res.push(Intersection {
                p,
                d,
                n: vector3::Vector3f::new(0.0, 1.0, 0.0),
                wo: &ray.direction * -1.0,
                u: p.x,
                v: p.z,
                dpdu: vector3::Vector3::new(1.0, 0.0, 0.0),
                dpdv: vector3::Vector3::new(0.0, 0.0, 1.0),
            });

            res
        }
    }

    /// A rectangle is an open surface and encloses nothing, so it contains no point.
    ///
    /// Answering `y <= 0` within the footprint would describe a solid reaching down for ever, and
    /// `get_bounding_box` below — flat at y = 0, correctly, since that is where the surface is —
    /// would then fail to contain it. Making the bound match instead would render every rectangle
    /// unbounded, and the floors, walls and light panels of every scene would leave the accelerator
    /// for a volume they do not have.
    fn contain_point(&self, _point: &Vector3f) -> bool {
        false
    }
}

impl AABound for Rectangle {
    /// The rectangle lies in the y = 0 plane, so its bound is flat along y.
    ///
    /// Flat and not padded, which rests on `AABoundingBox::hit` counting a tangential hit: padding
    /// the bound to, say, y ∈ [-1, 1] would be a two-unit-thick box around a surface with no
    /// thickness, loose enough to drag the rectangle into every node it does not belong to, and it
    /// would report an area it does not have to every split cost that reads it.
    fn get_bounding_box(&self) -> AABoundingBox {
        let bmin = Vector3f::new(-self.half_width, 0.0, -self.half_height);
        let bmax = Vector3f::new(self.half_width, 0.0, self.half_height);
        AABoundingBox::new(&bmin, &bmax)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::geom::vector2::Vector2u;
    use crate::samplers::{IndependentSampler, Sampler};

    /// An open surface encloses nothing, so no point is inside it — and its flat bound is then a
    /// truthful description rather than one that omits a volume.
    #[test]
    fn test_an_open_surface_encloses_nothing() {
        let rectangle = Rectangle::new(2.0, 3.0);

        assert!(!rectangle.contain_point(&Vector3f::new(0.0, -1.0, 0.0)));
        assert!(!rectangle.contain_point(&Vector3f::new(0.0, 0.0, 0.0)));

        let bbox = rectangle.get_bounding_box();
        assert_eq!(bbox.bmin.y, 0.0);
        assert_eq!(bbox.bmax.y, 0.0);
        assert!(bbox.is_bounded());
    }

    /// Sides of different lengths, and neither a round number of the other, so that a transposed
    /// half-side or a factor of two cannot pass unnoticed.
    const WIDTH: f64 = 3.0;
    const HEIGHT: f64 = 5.0;

    /// 100 000 draws leave the estimators below within a few tenths of a percent of their exact
    /// value, so a percent is loose enough never to fail by chance and tight enough to catch the
    /// errors that matter — all of which are factors, not fractions of a percent.
    const SAMPLES: usize = 100_000;
    const TOLERANCE: f64 = 0.01;

    fn sampler() -> IndependentSampler {
        // A fixed seed, so this test either passes or fails, always the same way.
        IndependentSampler::new(0, &Vector2u::new(0, 0), 0)
    }

    /// The Monte-Carlo estimator of the area, `Σ 1/pdf / N`, must converge to [`area`].
    ///
    /// The density is constant, so this does not test that the draw is *uniform* — the test below
    /// does. What it catches is the error that uniformity cannot show: a density off by a constant
    /// factor from the area it is supposed to be the reciprocal of. That error is invisible on any
    /// image, because it scales the light a source emits without changing anything about its shape,
    /// and it is exactly what a change of measure done twice, or half, would produce.
    #[test]
    fn test_the_density_integrates_to_the_area() {
        let rectangle = Rectangle::new(WIDTH, HEIGHT);
        let mut sampler = sampler();

        let mut total = 0.0;
        for _ in 0..SAMPLES {
            total += 1.0 / rectangle.sample_area(&sampler.get_2d()).pdf;
        }

        let estimate = total / SAMPLES as f64;
        let exact = WIDTH * HEIGHT;
        assert!(
            (estimate / exact - 1.0).abs() < TOLERANCE,
            "area estimated at {} against an exact {}",
            estimate,
            exact
        );
    }

    /// The draw is uniform over the surface, tested against integrals a constant cannot satisfy.
    ///
    /// `Σ f(p)/pdf / N` estimates `∫ f dA` for any `f`, and `f = 1` degenerates into the test
    /// above. Two moments are needed, and neither alone would do.
    ///
    /// **The first says where the points are centred.** The rectangle is centred on the origin, so
    ///
    /// ```text
    /// ∫∫ x dx dz = 0        and        ∫∫ z dx dz = 0
    /// ```
    ///
    /// This is the one that catches a draw landing on the wrong half — `u.x · half_width` instead
    /// of `(2·u.x − 1) · half_width` covers [0, a] rather than [-a, a], which is uniform, on the
    /// surface, and wrong.
    ///
    /// **The second says how far they spread.** Over [-a, a] × [-b, b],
    ///
    /// ```text
    /// ∫∫ x² dx dz = 2b · 2a³/3 = area · a²/3        and likewise ∫∫ z² dx dz = area · b²/3
    /// ```
    ///
    /// with `a = width/2` and `b = height/2`. Taking both coordinates is what catches them being
    /// swapped, the sides being different lengths.
    ///
    /// Note that the second moment alone could not catch the first error: `x²` is even, so [0, a]
    /// and [-a, a] give it the very same value.
    #[test]
    fn test_the_draw_is_uniform_over_the_surface() {
        let rectangle = Rectangle::new(WIDTH, HEIGHT);
        let mut sampler = sampler();
        let (half_width, half_height) = (WIDTH / 2.0, HEIGHT / 2.0);
        let area = WIDTH * HEIGHT;

        let (mut mean_x, mut mean_z, mut second_x, mut second_z) = (0.0, 0.0, 0.0, 0.0);
        for _ in 0..SAMPLES {
            let sample = rectangle.sample_area(&sampler.get_2d());
            let (x, z) = (sample.sp.p.x, sample.sp.p.z);

            mean_x += x / sample.pdf;
            mean_z += z / sample.pdf;
            second_x += x * x / sample.pdf;
            second_z += z * z / sample.pdf;
        }

        let n = SAMPLES as f64;

        // The exact value is zero, so the departure is measured against the half-side rather than
        // against itself: a centroid off by a percent of the extent is what the assertion allows.
        for (estimate, half_side, axis) in [(mean_x / n / area, half_width, "x"), (mean_z / n / area, half_height, "z")] {
            assert!(
                estimate.abs() < TOLERANCE * half_side,
                "the centroid sits at {} on {}, and the surface is centred on the origin",
                estimate,
                axis
            );
        }

        for (estimate, exact, axis) in [
            (second_x / n, area * half_width * half_width / 3.0, "x"),
            (second_z / n, area * half_height * half_height / 3.0, "z"),
        ] {
            assert!(
                (estimate / exact - 1.0).abs() < TOLERANCE,
                "∫{}² dA estimated at {} against an exact {}",
                axis,
                estimate,
                exact
            );
        }
    }

    /// A point *drawn* on the surface and a point *hit* by a ray are the same point, named the same
    /// way.
    ///
    /// This is the property an area light rests on, and the (u, v) half of it is the one that can
    /// go wrong in silence: `intersect` reports them unnormalised, `u = p.x` and `v = p.z`, so a
    /// `sample_area` mapping them to [0, 1) would agree on the position and disagree on the texture
    /// coordinates. Next-event estimation would then light a scene from one part of a texture while
    /// the eye saw another.
    #[test]
    fn test_a_drawn_point_is_the_point_a_ray_finds_there() {
        let rectangle = Rectangle::new(WIDTH, HEIGHT);
        let mut sampler = sampler();

        for _ in 0..1000 {
            let sample = rectangle.sample_area(&sampler.get_2d());

            // Straight down the normal, from far enough away that the ray has to travel.
            let origin = sample.sp.p + sample.sp.n * 2.0;
            let ray = Ray::spawn_from_through(&origin, &sample.sp.p);
            let hits = rectangle.intersect(&ray, 0.0, 100.0);

            assert_eq!(hits.len(), 1, "a ray aimed at {} must meet the surface", sample.sp.p);
            let hit = hits[0];

            assert!((hit.p - sample.sp.p).length() < 1e-12, "hit at {}, drawn at {}", hit.p, sample.sp.p);
            assert_eq!(hit.n, sample.sp.n, "normal");
            assert!((hit.u - sample.sp.u).abs() < 1e-12, "u: hit {}, drawn {}", hit.u, sample.sp.u);
            assert!((hit.v - sample.sp.v).abs() < 1e-12, "v: hit {}, drawn {}", hit.v, sample.sp.v);
        }
    }
}
