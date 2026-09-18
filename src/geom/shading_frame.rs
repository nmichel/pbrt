//! The orthonormal frame a surface interaction defines, and the change of basis into it.
//!
//! Reference: PBR Book, 3ed, chapter 8 — *Reflection Models*, « Geometric Setting ».
//! <https://www.pbr-book.org/3ed-2018/Reflection_Models#x0-GeometricSetting>
//!
//! # Frame
//!
//! This is **the** shading frame of the project, and the single place its convention is stated.
//! The basis is built from an [`Intersection`]:
//!
//! - `ns` = the surface normal `n`,
//! - `ss` = ∂p/∂u normalised — a tangent, which fixes the azimuth φ = 0,
//! - `ts` = ns × ss.
//!
//! **`z` is the normal.** A direction expressed in this frame therefore has `w.z = n⋅w = cos θ`,
//! which is what every BSDF in the project relies on: the polar angle of a direction is read off
//! one component instead of a dot product, and the hemisphere it belongs to off that component's
//! sign. Directions point *away* from the surface — `wo` away towards the viewer, `wi` away
//! towards the light — so both have positive `z` above the horizon.
//!
//! Angles are measured from `ns`, not from the geometric normal of the shape: nothing here knows
//! whether the two differ.
//!
//! # Preconditions
//!
//! `n` is assumed **unit**, and ∂p/∂u orthogonal to it. Neither is checked, and neither is
//! renormalised here: `ns` is taken as it comes, so a shape reporting a non-unit normal gets a
//! basis that is not orthonormal, and for it the round trip below is no longer the identity.
//! Normalising `ns` defensively would hide that in every shape rather than fix it in the one at
//! fault.
//!
//! `ts` needs no normalisation of its own: the cross product of two orthogonal unit vectors is
//! already unit.
//!
//! # The change of basis
//!
//! The columns of the local-to-world matrix are the basis vectors expressed in world
//! coordinates — that is what "expressing a local vector in world coordinates" means, since the
//! local frame's own axes are (1,0,0), (0,1,0), (0,0,1):
//!
//! ```text
//!                  |ss.x ts.x ns.x|
//! local_to_world = |ss.y ts.y ns.y|            wp = local_to_world ⋅ lp
//!                  |ss.z ts.z ns.z|
//! ```
//!
//! The inverse is the transpose, and this is worth spelling out rather than asserting: a matrix
//! whose columns are orthonormal satisfies Mᵀ⋅M = I, because entry (i, j) of that product is the
//! dot product of columns i and j — 1 on the diagonal, 0 off it. So Mᵀ *is* M⁻¹, and the reverse
//! change of basis costs three dot products instead of an inversion:
//!
//! ```text
//!                  |ss.x ss.y ss.z|
//! world_to_local = |ts.x ts.y ts.z|            lp = world_to_local ⋅ wp
//!                  |ns.x ns.y ns.z|
//! ```
//!
//! # Why a type rather than two methods on `Intersection`
//!
//! The basis costs a normalisation and a cross product to build, and a material converts two or
//! three directions per scatter. Holding it makes that cost paid once, but the reason it is a type
//! is that the frame is a *thing* a surface point has — and a thing can carry the convention
//! above, where two conversion functions could only repeat it.

use super::intersectable::Intersection;
use super::vector3;
use super::vector3::Vector3f;

pub struct ShadingFrame {
    ss: Vector3f,
    ts: Vector3f,
    ns: Vector3f,
}

/// The frame an intersection defines, built from its normal and ∂p/∂u.
///
/// A conversion rather than a named constructor, because an intersection determines its frame
/// entirely: there is no second way to build one from the same surface point, and no argument to
/// choose. The rest of the intersection — position, distance, texture coordinates, ∂p/∂v — has no
/// bearing on the basis and is dropped.
impl From<&Intersection> for ShadingFrame {
    fn from(intersection: &Intersection) -> Self {
        let ns = intersection.n;
        let mut ss = intersection.dpdu;
        ss.normalize();
        let ts = vector3::cross(&ns, &ss);

        Self { ss, ts, ns }
    }
}

impl ShadingFrame {
    /// `v`, given in world coordinates, expressed in this frame.
    pub fn world_to_local(&self, v: &Vector3f) -> Vector3f {
        Vector3f::new(vector3::dot(&self.ss, v), vector3::dot(&self.ts, v), vector3::dot(&self.ns, v))
    }

    /// `v`, given in this frame, expressed in world coordinates.
    pub fn local_to_world(&self, v: &Vector3f) -> Vector3f {
        Vector3f::new(
            self.ss.x * v.x + self.ts.x * v.y + self.ns.x * v.z,
            self.ss.y * v.x + self.ts.y * v.y + self.ns.y * v.z,
            self.ss.z * v.x + self.ts.z * v.y + self.ns.z * v.z,
        )
    }
}

/// The mirror direction of `wo` about the normal.
///
/// # Frame
///
/// `wo` is **already in the shading frame** — nothing here checks that, and nothing can: the
/// frame is a convention about what the components mean, not a property of the type. Passing a
/// world-space direction returns a mirror about the world z axis, silently.
///
/// # Derivation
///
/// `wo` points *away* from the intersection point, so the incident direction is wi = −wo. The
/// general reflection of wi about a normal n is
///
/// ```text
/// [1]   wi − 2(wi⋅n)n
/// ```
///
/// In this frame n = (0, 0, 1), so wi⋅n = wi_z and `[1]` only touches the third component:
///
/// ```text
/// [2]   (wi_x, wi_y, wi_z) − 2·wi_z·(0, 0, 1) = (wi_x, wi_y, −wi_z)
/// ```
///
/// Substituting wi = −wo gives (−wo_x, −wo_y, wo_z): the tangential components flip and the
/// normal component is kept. `test_reflect_agrees_with_the_general_formula` is `[1]` and this
/// result checked against each other.
pub fn reflect(wo: &Vector3f) -> Vector3f {
    Vector3f::new(-wo.x, -wo.y, wo.z)
}

/// Whether two directions lie on the same side of the surface.
///
/// # Frame
///
/// Both directions are **already in the shading frame**, where cos θ is the z component, so this
/// is the sign of cos θ₁·cos θ₂. Same caveat as [`reflect`]: the precondition is a convention, not
/// a checked property.
///
/// A direction exactly in the tangent plane (`z == 0`) belongs to *no* hemisphere and answers
/// `false` against everything, itself included — hence the strict `>`. That is the useful answer
/// for a BRDF: a grazing direction carries no energy, and `>=` would have it reflect.
pub fn same_hemisphere(w1: &Vector3f, w2: &Vector3f) -> bool {
    w1.z * w2.z > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An intersection whose frame is aligned with no axis, so that a conversion which silently
    /// did nothing — or swapped two components — could not pass.
    fn skewed_intersection() -> Intersection {
        let n = Vector3f::new(1.0, 2.0, 3.0).normalized();

        // A tangent: any vector orthogonal to n. Taking one that is not itself unit also checks
        // that the conversion normalises ∂p/∂u.
        let dpdu = vector3::cross(&n, &Vector3f::new(0.0, 0.0, 1.0)) * 7.0;

        Intersection {
            p: Vector3f::new(-1.0, 4.0, 0.5),
            d: 2.0,
            n,
            wo: Vector3f::new(0.0, 1.0, 0.0),
            u: 0.25,
            v: 0.75,
            dpdu,
            dpdv: vector3::cross(&n, &dpdu),
        }
    }

    const EPSILON: f64 = 1e-12;

    fn assert_close(actual: &Vector3f, expected: &Vector3f, label: &str) {
        let delta = (actual - expected).length();
        assert!(delta < EPSILON, "{}: {} != {} (delta {})", label, actual, expected, delta);
    }

    /// The two conversions are inverse of each other — the property the whole shading pipeline
    /// rests on, since a material converts into the frame, works there, and converts back out.
    #[test]
    fn test_round_trip_is_the_identity() {
        let frame = ShadingFrame::from(&skewed_intersection());

        let directions = [
            Vector3f::new(1.0, 0.0, 0.0),
            Vector3f::new(0.0, 1.0, 0.0),
            Vector3f::new(0.0, 0.0, 1.0),
            Vector3f::new(0.3, -0.5, 0.81).normalized(),
            Vector3f::new(-2.0, 7.0, -3.0),
        ];

        for v in directions.iter() {
            assert_close(&frame.local_to_world(&frame.world_to_local(v)), v, "world → local → world");
            assert_close(&frame.world_to_local(&frame.local_to_world(v)), v, "local → world → local");
        }
    }

    /// Mᵀ == M⁻¹ only holds for an orthonormal basis, which is the argument the module header
    /// makes before exploiting it.
    #[test]
    fn test_basis_is_orthonormal() {
        let frame = ShadingFrame::from(&skewed_intersection());

        for (v, name) in [(frame.ss, "ss"), (frame.ts, "ts"), (frame.ns, "ns")].iter() {
            assert!((v.length() - 1.0).abs() < EPSILON, "{} is not unit: {}", name, v.length());
        }

        for (u, v, names) in [
            (frame.ss, frame.ts, "ss⋅ts"),
            (frame.ts, frame.ns, "ts⋅ns"),
            (frame.ns, frame.ss, "ns⋅ss"),
        ]
        .iter()
        {
            assert!(vector3::dot(u, v).abs() < EPSILON, "{} is not zero: {}", names, vector3::dot(u, v));
        }
    }

    /// The convention itself, made executable: in this frame the normal *is* the z axis, so
    /// `w.z` is cos θ.
    #[test]
    fn test_normal_maps_to_the_z_axis() {
        let intersection = skewed_intersection();
        let frame = ShadingFrame::from(&intersection);

        assert_close(&frame.world_to_local(&intersection.n), &Vector3f::new(0.0, 0.0, 1.0), "n in local frame");
        assert_close(
            &frame.local_to_world(&Vector3f::new(0.0, 0.0, 1.0)),
            &intersection.n,
            "z axis in world frame",
        );
    }

    /// A direction's polar angle read in the frame agrees with the dot product against the normal
    /// taken in world coordinates. This is what licenses `w.z` to be used as cos θ everywhere.
    #[test]
    fn test_z_component_is_the_cosine_with_the_normal() {
        let intersection = skewed_intersection();
        let frame = ShadingFrame::from(&intersection);

        let w = Vector3f::new(0.3, -0.5, 0.81).normalized();
        let cos_theta_in_frame = frame.world_to_local(&w).z;
        let cos_theta_in_world = vector3::dot(&w, &intersection.n);

        assert!(
            (cos_theta_in_frame - cos_theta_in_world).abs() < EPSILON,
            "cos θ disagrees: {} in frame, {} in world",
            cos_theta_in_frame,
            cos_theta_in_world
        );
    }

    /// The derivation of [`reflect`], made executable: the three-component form in the frame and
    /// the general `wi − 2(wi⋅n)n` in world coordinates must be the same direction. A frame
    /// aligned with no axis is what gives the test teeth — on an axis-aligned one, a mistake in
    /// the basis would cancel out.
    #[test]
    fn test_reflect_agrees_with_the_general_formula() {
        let intersection = skewed_intersection();
        let frame = ShadingFrame::from(&intersection);
        let n = intersection.n;

        for wo in [
            Vector3f::new(0.3, -0.5, 0.81).normalized(),
            n,
            (n + Vector3f::new(0.4, 0.1, -0.2)).normalized(),
        ]
        .iter()
        {
            // The incident direction points at the surface, where `wo` points away from it.
            let wi = wo * -1.0;
            let in_world = wi - n * (2.0 * vector3::dot(&wi, &n));

            let in_frame = frame.local_to_world(&reflect(&frame.world_to_local(wo)));

            assert_close(&in_frame, &in_world, "reflected direction");
        }
    }

    /// Reflecting twice returns the original direction, and reflection is an isometry — the two
    /// properties any mirror must have, and the cheapest guard against a sign slip.
    #[test]
    fn test_reflect_is_an_involution_and_an_isometry() {
        let wo = Vector3f::new(0.3, -0.5, 0.81).normalized();

        assert_close(&reflect(&reflect(&wo)), &wo, "reflected twice");
        assert!((reflect(&wo).length() - 1.0).abs() < EPSILON, "reflection changed the length");
    }

    /// A mirror direction stays on the side it came from: `wo` and its reflection share an
    /// hemisphere, which is why [`reflect`] needs no horizon test of its own.
    #[test]
    fn test_reflect_keeps_the_hemisphere() {
        let above = Vector3f::new(0.3, -0.5, 0.81).normalized();
        let below = Vector3f::new(0.3, -0.5, -0.81).normalized();

        assert!(same_hemisphere(&above, &reflect(&above)));
        assert!(same_hemisphere(&below, &reflect(&below)));
    }

    /// The convention `same_hemisphere` encodes: only the sign of `z` matters, and a grazing
    /// direction is in no hemisphere at all.
    #[test]
    fn test_same_hemisphere() {
        let up = Vector3f::new(0.1, 0.2, 1.0);
        let also_up = Vector3f::new(-0.9, 0.7, 0.3);
        let down = Vector3f::new(0.1, 0.2, -1.0);
        let grazing = Vector3f::new(0.6, -0.8, 0.0);

        assert!(same_hemisphere(&up, &also_up), "two directions above the surface");
        assert!(same_hemisphere(&down, &(down * 2.0)), "two directions below the surface");
        assert!(!same_hemisphere(&up, &down), "opposite sides");

        // The tangent plane belongs to neither side — including against itself.
        assert!(!same_hemisphere(&grazing, &up), "grazing against up");
        assert!(!same_hemisphere(&grazing, &down), "grazing against down");
        assert!(!same_hemisphere(&grazing, &grazing), "grazing against itself");

        // x and y carry no information: only z decides.
        let up_elsewhere = Vector3f::new(-5.0, 3.0, up.z);
        assert!(same_hemisphere(&up, &up_elsewhere), "tangential components are irrelevant");
    }
}
