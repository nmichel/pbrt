//! A location on a surface: where it is, where it sits in the surface's own parameterisation, and
//! which way the surface faces there.
//!
//! Reference: PBR Book, 4ed, §3.11 — *Geometry and Transformations*, « Interactions ».
//! <https://pbr-book.org/4ed/Geometry_and_Transformations/Interactions>
//!
//! # The four fields
//!
//! - `p` — the position, in world coordinates.
//! - `n` — the unit normal, taken as the shape reports it. Which way it points on an open surface
//!   is that shape's convention, and nothing here re-reads or renormalises it.
//! - `u`, `v` — the coordinates of the point in the parameterisation the shape defines. Their range
//!   and their meaning belong to the shape — a sphere maps them to φ/2π and θ/π, a rectangle to its
//!   two sides — so nothing here wraps or clamps them either.
//!
//! # What a surface point is not
//!
//! An [`Intersection`] carries four fields more, and each is left out for its own reason.
//!
//! `d` and `wo` describe the **ray that found** the point, not the point. A point one *draws* on a
//! surface — uniformly over its area, say, to estimate an integral over that surface — is a surface
//! point with no ray behind it: no distance to report, and no direction to look back along.
//! Carrying the two fields would oblige such a caller to invent values for them, and any value it
//! invented would be one a reader could act on.
//!
//! ∂p/∂u and ∂p/∂v are the tangent basis, and
//! [`ShadingFrame`](super::shading_frame::ShadingFrame) is already the type that holds that basis
//! together with the convention it obeys. A second type carrying the same two vectors would be a
//! second place for that convention to be stated, hence a second place for it to be stated
//! differently.
//!
//! # Why a point rather than a pair of coordinates
//!
//! A texture that reads texture coordinates and nothing else would be served by (u, v) alone. That
//! is not what a texture *is* a function of: a value varying with world position is evaluated at
//! `p`, and one fading at grazing incidence against `n`. Narrowing the argument to the coordinates
//! would make it the intersection of what the implementations at hand happen to read, which is not
//! a concept, and which the next implementation would have to widen.
//!
//! # The cost
//!
//! Each lookup builds one of these where it could borrow an [`Intersection`] the caller already
//! holds: four fields copied, sixty-four bytes, on the shading path. Spending nothing would mean
//! making `Intersection` *hold* a `SurfacePoint` and handing out a borrow of that field — the same
//! four fields, bought by rewriting every site that builds an intersection. That is the trade
//! declined here: the copy is what the narrower argument costs, and it is paid where the argument
//! is narrowed.

use super::intersectable::Intersection;
use super::vector3::Vector3f;

#[derive(Debug, Clone, Copy)]
pub struct SurfacePoint {
    /// Position, in world coordinates.
    pub p: Vector3f,

    /// Unit normal, as the shape reports it.
    pub n: Vector3f,

    /// Coordinates in the parameterisation the shape defines.
    pub u: f64,
    pub v: f64,
}

/// The surface point an intersection landed on.
///
/// A conversion rather than a named constructor, for the reason
/// [`ShadingFrame`](super::shading_frame::ShadingFrame) gives: an intersection determines its
/// surface point entirely, there is no second one to build from it, and there is no argument to
/// choose. `d`, `wo`, ∂p/∂u and ∂p/∂v have no bearing on it and are dropped; the module header says
/// why each of them is.
impl From<&Intersection> for SurfacePoint {
    fn from(intersection: &Intersection) -> Self {
        Self {
            p: intersection.p,
            n: intersection.n,
            u: intersection.u,
            v: intersection.v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every field of the intersection distinct from every other, so that a conversion writing one
    /// where another belongs cannot pass.
    ///
    /// **This test is the only guard the conversion has**, and no rendered image can stand in for
    /// it. No texture in the project reads `p` or `n`, so a mistake in either is invisible on every
    /// scene; and [`CheckerBoard`](crate::textures::CheckerBoard) is symmetric in its two
    /// coordinates — swapping `u` and `v` leaves each of its two comparisons unchanged — so even a
    /// transposition is invisible on the one texture that reads them.
    ///
    /// Exact equality is the right comparison: this is a copy, and a copy off by an epsilon is not
    /// one.
    #[test]
    fn test_each_field_comes_from_its_namesake() {
        let intersection = Intersection {
            p: Vector3f::new(1.0, 2.0, 3.0),
            d: 4.0,
            n: Vector3f::new(5.0, 6.0, 7.0),
            wo: Vector3f::new(8.0, 9.0, 10.0),
            u: 0.25,
            v: 0.75,
            dpdu: Vector3f::new(11.0, 12.0, 13.0),
            dpdv: Vector3f::new(14.0, 15.0, 16.0),
        };

        // Destructured rather than read field by field: a field added to `SurfacePoint` without a
        // line here stops compiling instead of going unchecked.
        let SurfacePoint { p, n, u, v } = SurfacePoint::from(&intersection);

        assert_eq!(intersection.p, p, "position");
        assert_eq!(intersection.n, n, "normal");
        assert_eq!(intersection.u, u, "u");
        assert_eq!(intersection.v, v, "v");
    }
}
