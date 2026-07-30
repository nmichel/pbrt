use super::ray::Ray;
use super::transform::{Transform, Transformable};
use super::vector3::Vector3f;
use std::mem::swap;

#[derive(Debug, Clone, Copy)]
pub struct AABoundingBox {
    pub bmin: Vector3f,
    pub bmax: Vector3f,
}

/// Trait implemented by object that can be contained in a Axis Aligned Bounding Box.
/// Used by acceleration structure to effeciently search for collisions.
pub trait AABound {
    fn get_bounding_box(&self) -> AABoundingBox;
}

/// Unit roundoff for `f64`: u = ε/2 = 2⁻⁵³, where ε = `f64::EPSILON` = 2⁻⁵².
///
/// IEEE-754 guarantees that every elementary operation is correctly rounded, i.e.
/// `fl(a ⊙ b) = (a ⊙ b)(1 + δ)` with |δ| ≤ u.
///
/// Why the bound has to be *relative* — floats are spaced proportionally to their
/// magnitude, so no absolute epsilon can hold over the range — is worked out in
/// `docs/arithmetique_flottante.md` §0 and §1.
const UNIT_ROUNDOFF: f64 = f64::EPSILON * 0.5;

/// γ(n) = n·u / (1 − n·u), evaluated at n = 3.
///
/// Three is the number of rounded operations involved in one slab distance: the
/// subtraction, the reciprocal, and the multiplication. See `AABoundingBox::hit` for the
/// derivation, and `docs/arithmetique_flottante.md` §3 for what γ(n) bounds and why this
/// closed form is preferred to (1 + u)ⁿ − 1.
const GAMMA_3: f64 = 3.0 * UNIT_ROUNDOFF / (1.0 - 3.0 * UNIT_ROUNDOFF);

/// Relative half-width of the interval that provably brackets the exact slab distance,
/// from [4] in `AABoundingBox::hit`; `docs/arithmetique_flottante.md` §4 details the three
/// steps that lead to it, and why the factor is 2γ(3) rather than γ(3).
const SLAB_WIDENING: f64 = 2.0 * GAMMA_3;

impl AABoundingBox {
    /// An empty box, built so that `combine_with` yields the other operand unchanged.
    ///
    /// `bmin`/`bmax` are deliberately inverted: this is not a valid box and must not be
    /// passed to `hit` or `half_area` before being combined with at least one real box.
    pub fn new_invalid() -> Self {
        Self {
            bmin: Vector3f::max(),
            bmax: Vector3f::min(),
        }
    }

    /// Build a box from its two extreme corners.
    ///
    /// The bound is stored **faithfully**: a flat box stays flat, an empty box stays
    /// empty. This matters because acceleration structures derive their split cost from
    /// `half_area`, so inflating degenerate boxes here would silently bias every SAH
    /// evaluation — a box thickened to a floor value reports an area it does not have.
    /// Degeneracy is instead handled where it actually bites, in `hit`.
    pub fn new(lower: &Vector3f, higher: &Vector3f) -> Self {
        debug_assert!(
            lower.x <= higher.x && lower.y <= higher.y && lower.z <= higher.z,
            "AABoundingBox::new expects lower <= higher componentwise, got {:?} and {:?} (use new_invalid() for an empty box)",
            lower,
            higher
        );

        Self { bmin: *lower, bmax: *higher }
    }

    pub fn half_area(&self) -> f64 {
        let d = self.bmax - self.bmin;
        d.x * d.y + d.y * d.z + d.z * d.x
    }

    pub fn centroid(&self) -> Vector3f {
        (self.bmax + self.bmin) * 0.5
    }

    /// Slab-based ray/box intersection test.
    ///
    /// Returns the parametric distance at which `ray` enters the box, clamped to `tmin`
    /// (so a ray originating inside the box yields `tmin`), or `None` when the ray misses.
    ///
    /// Slab method of Kay & Kajiya, as presented in
    /// <https://raytracing.github.io/books/RayTracingTheNextWeek.html#boundingvolumehierarchies>,
    /// with the robustness treatment of
    /// <https://www.pbr-book.org/3ed-2018/Shapes/Basic_Shape_Interface#RayBoundsIntersections>.
    ///
    /// # Principle
    ///
    /// The box is the intersection of three axis-aligned slabs. Along axis i the ray
    /// crosses the two planes of the slab at
    ///
    /// ```text
    /// t = (bound[i] − o[i]) / d[i]
    /// ```
    ///
    /// Intersecting the three parametric intervals leaves [tmin, tmax]; the ray meets the
    /// box if and only if that interval is non-empty.
    ///
    /// # Why the rejection test is strict
    ///
    /// A bounding box may legitimately be flat: an axis-aligned triangle, a `Rectangle`
    /// and a `Plane` all have zero extent along one axis. For such a slab the two planes
    /// coincide, so t₀ == t₁ and the interval collapses to the single point tmin == tmax.
    /// That intersection has measure zero, but a bounding box is a *conservative* bound —
    /// rejecting it would discard the primitive it contains, which is a hole in the image.
    /// Tangency must therefore count as a hit, hence `tmax < tmin` and not `tmax <= tmin`.
    /// The same argument covers a ray grazing an edge or a face of a non-degenerate box.
    ///
    /// # Rays parallel to a slab
    ///
    /// When d[i] == 0 the expression above degenerates to x/0 or 0/0. Rather than lean on
    /// IEEE infinity and NaN propagation — where `f64::max` happens to drop the NaN and
    /// skip the axis, which is the right outcome reached by accident — the case is handled
    /// explicitly: a ray parallel to a slab never crosses either plane, so it lies within
    /// the slab exactly when its origin already does. The `== 0.0` test matches both +0.0
    /// and −0.0, as IEEE equality ignores the sign of zero.
    ///
    /// # Conservative rounding
    ///
    /// What u, δᵢ and γ(n) are, and why floating-point representation forces a *relative*
    /// bound in the first place, is laid out in `docs/arithmetique_flottante.md`.
    ///
    /// Each t above is computed with three correctly rounded operations: the subtraction
    /// bound[i] − o[i], the reciprocal 1/d[i], and the multiplication of the two. Note that
    /// n = 3 is a property of the code, not of the formula: written as a single division,
    /// (bound[i] − o[i]) / d[i] would only round twice and γ(2) would suffice. The
    /// reciprocal is pulled out of the quotient so that it can be shared by the two planes
    /// of the slab — which makes it a third rounding, and its error enters both t's.
    ///
    /// So with |δᵢ| ≤ u the computed value t̃ carries the product of three error factors:
    ///
    /// ```text
    /// t̃ = t(1 + δ₁)(1 + δ₂)(1 + δ₃)
    /// ```
    ///
    /// The standard bound on such a product (Higham, *Accuracy and Stability of Numerical
    /// Algorithms*, §3.1) is
    ///
    /// ```text
    /// |(1 + δ₁)…(1 + δₙ) − 1| ≤ γ(n),   γ(n) = n·u / (1 − n·u)            [1]
    /// ```
    ///
    /// Now *name* the whole product's deviation from 1 — this is a definition, nothing is
    /// being deduced:
    ///
    /// ```text
    /// e := (1 + δ₁)(1 + δ₂)(1 + δ₃) − 1
    /// ```
    ///
    /// Substituting that into the expression for t̃ gives t̃ = t(1 + e), and |e| ≤ γ(3) is
    /// [1] itself restated, since e is exactly the quantity [1] bounds. Expanded,
    ///
    /// ```text
    /// e = δ₁ + δ₂ + δ₃ + δ₁δ₂ + δ₁δ₃ + δ₂δ₃ + δ₁δ₂δ₃
    ///     └── order u ──┘   └──── order u² ───┘  └ u³ ┘
    /// ```
    ///
    /// so e is dominated by the sum of the three roundings — the intuition that three
    /// roundings drift about three times one — and the denominator of γ(3) is what covers
    /// the cross terms. Higham states his lemma directly in this `1 + θₙ` form; e is his
    /// θ₃.
    ///
    /// The point of the substitution is to collapse three independent unknowns into one,
    /// which is what makes the inversion tractable: from t̃ = t(1 + e) we get
    /// t = t̃ / (1 + e), a single-variable function of the one remaining unknown.
    ///
    /// ## Bracketing t from t̃
    ///
    /// 1/(1 + e) is strictly monotonic in e, so it has no interior extremum: over
    /// e ∈ [−γ, γ] the extreme values can only be at the two endpoints. Taking t̃ > 0 for
    /// now,
    ///
    /// ```text
    /// e = +γ  →  t = t̃/(1 + γ)      the smallest t can be
    /// e = −γ  →  t = t̃/(1 − γ)      the largest
    ///
    /// t ∈ [ t̃/(1 + γ) ,  t̃/(1 − γ) ]        exact, but full of divisions
    /// ```
    ///
    /// That bracket is already correct; what it is not is cheap. [2] and [3] trade the
    /// divisions for multiplications. Each is a deliberate *loosening*, and the whole
    /// point is that each one loosens **outwards** — the lower bound may only move down,
    /// the upper bound only up, or the bracket stops being one. That is what fixes the
    /// direction of each inequality:
    ///
    /// ```text
    /// t ≥ t̃/(1 + γ) needs something smaller:  1/(1 + γ) ≥ 1 − γ           [2]
    /// t ≤ t̃/(1 − γ) needs something larger:   1/(1 − γ) ≤ 1 + 2γ          [3]
    ///
    /// t ∈ [ t̃(1 − γ) ,  t̃(1 + 2γ) ]          multiplications, asymmetric
    /// ```
    ///
    /// [2] holds because (1 − γ)(1 + γ) = 1 − γ² ≤ 1, so dividing by (1 + γ) > 0 gives
    /// 1 − γ ≤ 1/(1 + γ). [3] holds because (1 + 2γ)(1 − γ) = 1 + γ − 2γ² = 1 + γ(1 − 2γ)
    /// ≥ 1 exactly when γ ≤ 1/2; with γ(3) ≈ 3.3·10⁻¹⁶ the condition is met by fifteen
    /// orders of magnitude.
    ///
    /// Two last widenings, both outwards again, both for uniformity rather than for
    /// correctness. First, symmetrise by taking the worse of the two sides, 2γ. Since
    /// 2γ > γ, this *lowers* the lower bound — t̃(1 − 2γ) < t̃(1 − γ) — so it deliberately
    /// gives up tightness. It costs nothing: weakening a lower bound can never make it
    /// false, and the widening stays in the last few ULP either way.
    ///
    /// ```text
    /// t ∈ [ t̃(1 − 2γ) ,  t̃(1 + 2γ) ]         symmetric
    /// ```
    ///
    /// What that buys is sign-agnosticism, and it is the whole reason for the step. The
    /// bracket above was derived assuming t̃ > 0; that assumption is what decided which
    /// endpoint was the minimum. For t̃ < 0 monotonicity swaps the two roles and the
    /// asymmetric form is simply wrong — with γ = 0.1 and t̃ = −5, t ranges over
    /// [−5.5556, −4.5455] while [t̃(1 − γ), t̃(1 + 2γ)] reads [−4.5, −6.0], which does not
    /// even bracket. The factors (1 − γ) and (1 + 2γ) are tied to a *role* that depends on
    /// the sign of t̃; once both sides carry 2γ, the roles collapse and only "move away by
    /// 2γ|t̃| on each side" remains. Writing that in magnitude is then possible, and yields
    /// a form no branch on the sign is needed for:
    ///
    /// ```text
    /// t ∈ [ t̃ − |t̃|·2γ(3) ,  t̃ + |t̃|·2γ(3) ]                              [4]
    /// ```
    ///
    /// Symmetrisation is therefore not an independent simplification: it is the
    /// precondition of the magnitude form. `docs/arithmetique_flottante.md` §4 works the
    /// three steps through with worked intervals.
    ///
    /// Each slab interval is widened outwards by that amount before being intersected:
    /// t₀ is lowered, t₁ raised. This can only turn a miss into a hit, never the reverse,
    /// which is the asymmetry we want — a false negative drops geometry from the image,
    /// whereas a false positive costs one redundant primitive test.
    ///
    /// pbrt widens t₁ alone, by a factor (1 + 2γ(3)). That is cheaper, but scaling is only
    /// a widening when t₁ > 0: for a negative t₁ the same factor moves the bound the wrong
    /// way, and the case is merely benign in practice because such a box lies behind the
    /// ray and is rejected against `tmin` anyway. We widen both ends in magnitude instead,
    /// which is conservative by construction whatever the signs, at the cost of two
    /// multiplications and two `abs` per axis.
    ///
    /// Note the scope of this bound: it covers the arithmetic of the slab test only. Error
    /// already present in the inputs — notably the normalisation of `ray.direction` — is a
    /// separate matter and is not accounted for here.
    pub fn hit(&self, ray: &Ray, mut tmin: f64, mut tmax: f64) -> Option<f64> {
        for i in 0..3 {
            if ray.direction[i] == 0.0 {
                // Parallel to this slab: inside it iff the origin is.
                if ray.origin[i] < self.bmin[i] || ray.origin[i] > self.bmax[i] {
                    return None;
                }
                continue;
            }

            let inv_dir = 1.0 / ray.direction[i];
            let mut t0 = (self.bmin[i] - ray.origin[i]) * inv_dir;
            let mut t1 = (self.bmax[i] - ray.origin[i]) * inv_dir;
            if inv_dir < 0.0 {
                swap(&mut t1, &mut t0);
            }

            // Widen the slab outwards by the rounding bound [4] above.
            t0 = t0 - t0.abs() * SLAB_WIDENING;
            t1 = t1 + t1.abs() * SLAB_WIDENING;

            tmin = f64::max(t0, tmin);
            tmax = f64::min(t1, tmax);

            if tmax < tmin {
                return None;
            }
        }

        Some(tmin)
    }

    /// Update self such as it encompass itself and other.
    ///
    /// # Example
    /// ```
    /// use pbrt::geom::aabound::AABoundingBox;
    /// use pbrt::geom::vector3::Vector3f;
    /// let mut a = AABoundingBox::new(&Vector3f::new(-1.0, -1.0, -1.0), &Vector3f::new(1.0, 1.0, 1.0));
    /// let b = AABoundingBox::new(&Vector3f::new(0.0, -1.0, -1.0), &Vector3f::new(2.0, 1.0, 1.0));
    /// a.combine_with(&b);
    /// assert_eq!(a.bmin.x, -1.0);
    /// assert_eq!(a.bmax.x, 2.0);
    /// ```
    pub fn combine_with(&mut self, other: &AABoundingBox) -> &mut Self {
        self.bmin.minimize_by(&other.bmin);
        self.bmax.maximize_by(&other.bmax);
        self
    }

    /// Return the AABoundingBox emcompassing a and b.
    ///
    /// # Example
    /// ```
    /// use pbrt::geom::aabound::AABoundingBox;
    /// use pbrt::geom::vector3::Vector3f;
    /// let a = AABoundingBox::new(&Vector3f::new(-1.0, -1.0, -1.0), &Vector3f::new(1.0, 1.0, 1.0));
    /// let b = AABoundingBox::new(&Vector3f::new(0.0, -1.0, -1.0), &Vector3f::new(2.0, 1.0, 1.0));
    /// let c = AABoundingBox::combine(&a, &b);
    /// assert_eq!(c.bmin.x, -1.0);
    /// assert_eq!(c.bmin.y, -1.0);
    /// assert_eq!(c.bmin.z, -1.0);
    /// assert_eq!(c.bmax.x, 2.0);
    /// assert_eq!(c.bmax.y, 1.0);
    /// assert_eq!(c.bmax.z, 1.0);
    /// ```
    pub fn combine(a: &AABoundingBox, b: &AABoundingBox) -> AABoundingBox {
        let mut res = AABoundingBox::new_invalid();
        res.combine_with(a).combine_with(b);
        res
    }
}

impl Transformable<AABoundingBox> for AABoundingBox {
    fn transform(&self, transform: &Transform) -> Self {
        let min = &self.bmin;
        let max = &self.bmax;
        let vertices = vec![
            Vector3f::new(min.x, min.y, min.z),
            Vector3f::new(min.x, min.y, max.z),
            Vector3f::new(min.x, max.y, min.z),
            Vector3f::new(min.x, max.y, max.z),
            Vector3f::new(max.x, max.y, max.z),
            Vector3f::new(max.x, max.y, min.z),
            Vector3f::new(max.x, min.y, max.z),
            Vector3f::new(max.x, min.y, min.z),
        ];

        let mut transformed_min = Vector3f::max();
        let mut transformed_max = Vector3f::min();

        for vertex in vertices.iter() {
            let transformed_point = transform.transform_point_to_world(vertex);
            transformed_min.minimize_by(&transformed_point);
            transformed_max.maximize_by(&transformed_point);
        }

        AABoundingBox::new(&transformed_min, &transformed_max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collide() {
        let bbox = AABoundingBox::new(&Vector3f::new(-1.0, -1.0, -1.0), &Vector3f::new(1.0, 1.0, 1.0));
        let tests = vec![
            // Rays parallel to axis
            //
            // Ray origin at frame origin
            //
            (Ray::new(&Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(0.0, 0.0, 1.0)), true),
            (Ray::new(&Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(0.0, 1.0, 0.0)), true),
            (Ray::new(&Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(1.0, 0.0, 0.0)), true),
            (Ray::new(&Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(0.0, 0.0, -1.0)), true),
            (Ray::new(&Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(0.0, -1.0, 0.0)), true),
            (Ray::new(&Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(-1.0, 0.0, 0.0)), true),
            // Ray origin outside the cube, such as ray doesn't intersect
            //
            (Ray::new(&Vector3f::new(1.01, 0.0, 0.0), &Vector3f::new(0.0, 0.0, 1.0)), false),
            (Ray::new(&Vector3f::new(0.0, 0.0, -1.1), &Vector3f::new(0.0, 1.0, 0.0)), false),
            (Ray::new(&Vector3f::new(0.0, 0.0, 1.01), &Vector3f::new(1.0, 0.0, 0.0)), false),
            (Ray::new(&Vector3f::new(-1.01, 0.0, 0.0), &Vector3f::new(0.0, 0.0, -1.0)), false),
            (Ray::new(&Vector3f::new(1.01, 0.0, -1.01), &Vector3f::new(0.0, -1.0, 0.0)), false),
            (Ray::new(&Vector3f::new(1.10, 1.01, 0.0), &Vector3f::new(-1.0, 0.0, 0.0)), false),
            // Ray origin inside the cube, diagonal rays
            //
            (Ray::new(&Vector3f::new(0.5, -0.5, 0.0), &Vector3f::new(1.0, -1.0, 1.0)), true),
            (Ray::new(&Vector3f::new(0.5, -0.5, 0.0), &Vector3f::new(-0.63, -1.0, 2.13)), true),
            (Ray::new(&Vector3f::new(0.5, -0.5, 0.0), &Vector3f::new(12.2, 0.004, -0.0003)), true),
            // Ray origin outside the cube, diagonal rays
            //
            (Ray::new(&Vector3f::new(-2.0, 0.0, 0.0), &Vector3f::new(3.0, 1.0, 0.0)), true),
            (Ray::new(&Vector3f::new(-2.0, 0.0, 0.0), &Vector3f::new(3.0, 1.0, -2.0)), true),
        ];

        for (ray, expected_res) in tests.iter() {
            if *expected_res {
                assert!(bbox.hit(ray, 0.0, 1000.0).is_some(), "Ray {:?} should hit the bounding box", ray);
            }
            else {
                assert!(bbox.hit(ray, 0.0, 1000.0).is_none(), "Ray {:?} should not hit the bounding box", ray);
            }
        }
    }

    /// A bounding box may be flat — an axis-aligned triangle, a `Rectangle`, a `Plane`.
    /// Since the box is a conservative bound, a tangential hit must be reported or the
    /// primitive it contains is dropped. This is what the strict comparison in `hit` buys:
    /// with the previous `tmax <= tmin`, every crossing case below was rejected.
    #[test]
    fn test_collide_flat_box() {
        // Zero extent along y, two units along x and z.
        let flat = AABoundingBox::new(&Vector3f::new(-1.0, 0.0, -1.0), &Vector3f::new(1.0, 0.0, 1.0));

        let crossing = vec![
            // Head-on through the plane, from either side.
            Ray::new(&Vector3f::new(0.0, 1.0, 0.0), &Vector3f::new(0.0, -1.0, 0.0)),
            Ray::new(&Vector3f::new(0.0, -1.0, 0.0), &Vector3f::new(0.0, 1.0, 0.0)),
            // Through the plane at an angle.
            Ray::new(&Vector3f::new(-0.5, 1.0, -0.5), &Vector3f::new(0.3, -1.0, 0.2)),
            // Lying exactly in the plane, origin within the footprint.
            Ray::new(&Vector3f::new(-0.5, 0.0, 0.0), &Vector3f::new(1.0, 0.0, 0.0)),
        ];
        for ray in crossing.iter() {
            assert!(flat.hit(ray, 0.0, 1000.0).is_some(), "Ray {:?} should hit the flat box", ray);
        }

        let missing = vec![
            // Parallel to the plane, offset from it.
            Ray::new(&Vector3f::new(-0.5, 0.5, 0.0), &Vector3f::new(1.0, 0.0, 0.0)),
            // Crosses the plane, but outside the footprint.
            Ray::new(&Vector3f::new(2.0, 1.0, 0.0), &Vector3f::new(0.0, -1.0, 0.0)),
            // Pointing away from the plane.
            Ray::new(&Vector3f::new(0.0, 1.0, 0.0), &Vector3f::new(0.0, 1.0, 0.0)),
        ];
        for ray in missing.iter() {
            assert!(flat.hit(ray, 0.0, 1000.0).is_none(), "Ray {:?} should miss the flat box", ray);
        }
    }

    /// A ray grazing a face of a non-degenerate box is tangential too, and must hit for
    /// the same conservativeness reason.
    #[test]
    fn test_collide_grazing_face() {
        let bbox = AABoundingBox::new(&Vector3f::new(-1.0, -1.0, -1.0), &Vector3f::new(1.0, 1.0, 1.0));

        // Sliding along the y = +1 face.
        let grazing = Ray::new(&Vector3f::new(-2.0, 1.0, 0.0), &Vector3f::new(1.0, 0.0, 0.0));
        assert!(bbox.hit(&grazing, 0.0, 1000.0).is_some(), "A ray grazing a face should hit");
    }

    /// `half_area` feeds the SAH split cost, so it must report the true area. Inflating a
    /// degenerate box — as the constructor used to do, with a 0.01 floor per axis — makes
    /// every cost comparison wrong.
    #[test]
    fn test_degenerate_box_area_is_faithful() {
        // d = (2, 0, 3), so half_area = dx·dy + dy·dz + dz·dx = 0 + 0 + 6.
        let flat = AABoundingBox::new(&Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(2.0, 0.0, 3.0));
        assert_eq!(flat.half_area(), 6.0);

        let point = AABoundingBox::new(&Vector3f::zero(), &Vector3f::zero());
        assert_eq!(point.half_area(), 0.0);
    }

    /// Guards the rounding constants derived in `hit`: γ(3) must stay a small multiple of
    /// the unit roundoff. If this ever grows to a macroscopic epsilon, the widening has
    /// turned back into the box inflation it replaced.
    #[test]
    fn test_rounding_bound_magnitude() {
        assert!(GAMMA_3 > 3.0 * UNIT_ROUNDOFF, "γ(3) must exceed 3u");
        assert!(GAMMA_3 < 4.0 * UNIT_ROUNDOFF, "γ(3) must stay well under 4u");
        assert!(SLAB_WIDENING < 1e-15, "the slab widening must remain negligible");
    }
}
