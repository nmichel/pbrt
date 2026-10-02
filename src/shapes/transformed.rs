//! A shape placed in the world, and the one place a *placement* of geometry lives.
//!
//! # What it is for
//!
//! A shape is built in a space of its own: a [`Sphere`](super::Sphere) sits at the origin, a
//! [`Rectangle`](super::Rectangle) lies in the y = 0 plane. This decorator is what puts such a
//! shape somewhere — the loader folds the current transformation matrix into the geometry here, so
//! that whoever holds the shape afterwards reads world-space points without having to agree on a
//! placement of their own.
//!
//! That last clause is the whole reason the decorator sits at the shape level rather than only at
//! the object level: a surface is about to be looked at by two consumers at once — the visible
//! object that shades it, and the light that samples the same surface for illumination. Placing the
//! geometry itself lets them share one `Arc<dyn Shape>` and see the same points, where a placement
//! wrapped around a finished object leaves the light holding the unplaced shape.
//!
//! # Frame
//!
//! `to_world` maps the child's own space to world space, and the child is the only thing that lives
//! in the former: every value crossing this type's interface is in **world space**. The
//! conversions therefore come in pairs, one per direction of travel:
//!
//! - a ray travels *in*, world → local, before the child is asked anything;
//! - a hit travels *out*, local → world, before it is handed back.
//!
//! Placing a hit is delegated to [`Transform::transform_interaction_to_world`], which knows that a
//! normal does not transform like a direction — it needs the inverse transpose, or it stops being
//! perpendicular to the surface under any transform that is not a rotation.
//!
//! # Precondition: the placement preserves distances along a ray
//!
//! `near`, `far` and the hit distance `d` all cross unchanged, in both directions. They are
//! parametric distances along the ray, and that is only sound while the placement preserves
//! them — which holds for the rotations and translations `Transform` offers, and fails for a
//! scaling one.
//!
//! The failure would be silent, and worth spelling out because it is not the usual "the matrix is
//! not an isometry" argument: [`Ray::new`] normalises its direction, so the local ray handed to the
//! child is unit even when the placement scaled it. One unit of local travel is then one local
//! unit rather than one world unit, and the two rulers differ by the scale factor. `d` comes back
//! measured with the wrong one, and `near`/`far` went in measured with the wrong one too — so a
//! scaled placement would report hits at distances that are not the distances the caller asked
//! about. [`objects::Transformed`](crate::objects::Transformed) inherits the same assumption; this
//! decorator adds none.
//!
//! # No round trip is saved
//!
//! The ray journey here is exactly the one an object-level placement performs, one storey lower. A
//! placed shape is not cheaper to intersect than a placed object, and this decorator must not be
//! credited with a speed argument it does not have: it exists so that geometry can be shared in
//! world space, and for nothing else.

use std::sync::Arc;

use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::surface_point::SurfacePoint;
use crate::geom::transform::{Transform, Transformable};
use crate::geom::vector2::Vector2f;
use crate::geom::vector3::Vector3f;

use super::{AreaSampleable, Shape, ShapeSample, SolidAngleSample};

/// A shape and the transformation that places it in the world.
pub struct Transformed {
    shape: Arc<dyn Shape>,
    to_world: Box<Transform>,
}

impl Transformed {
    pub fn new(shape: Arc<dyn Shape>, to_world: Box<Transform>) -> Self {
        Self { shape, to_world }
    }
}

impl Shape for Transformed {
    /// The child's area-sampling face, placed.
    ///
    /// **This relay is not optional.** A scene wraps every shape in this decorator, so this is the
    /// object the question is put to — not the [`Rectangle`](super::Rectangle) inside it. Left to
    /// the default, it would answer `None` for a shape that plainly can be sampled, and every
    /// placed emitter would look unsamplable.
    ///
    /// The child is asked once, here, rather than at every draw: the answer is a handle, and
    /// probing through `Arc<dyn Shape>` on the sampling path would clone a reference count per
    /// point drawn.
    fn area_sampler(self: Arc<Self>) -> Option<Arc<dyn AreaSampleable>> {
        let local_sampler = Arc::clone(&self.shape).area_sampler()?;

        Some(Arc::new(PlacedAreaSampler {
            shape: local_sampler,
            to_world: (*self.to_world).clone(),
        }))
    }
}

/// A shape's area-sampling face, wearing the placement of the [`Transformed`] it came from.
///
/// It holds its own copy of the transformation — a [`Transform`] is a value, and so is cheap to
/// own — rather than borrowing from the decorator. Nothing else would let the returned handle
/// outlive the call that asked for it.
struct PlacedAreaSampler {
    shape: Arc<dyn AreaSampleable>,
    to_world: Transform,
}

impl AreaSampleable for PlacedAreaSampler {
    /// The child's area, unchanged.
    ///
    /// This rests on the precondition of this module: the placement preserves distances, which is
    /// true of the rotations and translations [`Transform`] offers. An isometry maps a surface onto
    /// one of the same area, so there is nothing to scale. A scaling placement would break this the
    /// way it already breaks the ray distances above — and it would do so silently, since an area
    /// off by a factor changes how much light a source emits without changing anything one can see
    /// about its shape.
    fn area(&self) -> f64 {
        self.shape.area()
    }

    /// The child's point, placed — the position and the normal travel out, as they do for a hit.
    ///
    /// The texture coordinates cross unchanged: they name a place in the surface's own
    /// parameterisation, which a placement does not touch. So does the density, and for the reason
    /// [`area`](Self::area) gives — it is expressed in area measure, and the area is the same on
    /// both sides of an isometry.
    fn sample_area(&self, u: &Vector2f) -> ShapeSample {
        let local = self.shape.sample_area(u);

        ShapeSample {
            sp: SurfacePoint {
                p: self.to_world.transform_point_to_world(&local.sp.p),
                n: self.to_world.transform_normal_to_world(&local.sp.n),
                u: local.sp.u,
                v: local.sp.v,
            },
            pdf: local.pdf,
        }
    }

    /// The child's draw, with the reference point carried **into** its space and the result brought
    /// back out.
    ///
    /// The trait's default body would work here without a line of this, and would be correct: it
    /// draws by area — which this type already places — and converts in world space. What it would
    /// *not* do is let the child draw in its own solid angle, because the child has never been told
    /// where anything is looked at from. So the one thing this adds is the journey of `reference`,
    /// and it is the whole reason the override exists.
    ///
    /// **The density crosses unchanged**, and that rests on the precondition of this module: the
    /// placement preserves distances. An isometry maps a cone onto a cone of the same opening, so a
    /// density per unit solid angle is the same number read from either side. A scaling placement
    /// would break this exactly as it already breaks [`area`](Self::area), and as quietly.
    ///
    /// `wi` **crosses the placement** like the direction it is, and is on no account recomputed as
    /// `(p − reference).normalized()` on this side. The two are the same direction in exact
    /// arithmetic, a rotation taking a unit vector to a unit vector — but the subtraction cancels
    /// to nothing whenever the drawn point is the reference point, which happens every time a path
    /// runs next event estimation from a vertex lying on this very lamp. It came back `NaN`, one
    /// sample in two hundred, and a pixel that met one stayed black: see the note on a reference
    /// sitting on the surface in [`Sphere::sample_solid_angle`](super::Sphere).
    fn sample_solid_angle(&self, reference: &Vector3f, u: &Vector2f) -> Option<SolidAngleSample> {
        let local_reference = self.to_world.transform_point_to_local(reference);
        let local = self.shape.sample_solid_angle(&local_reference, u)?;

        Some(SolidAngleSample {
            sp: SurfacePoint {
                p: self.to_world.transform_point_to_world(&local.sp.p),
                n: self.to_world.transform_normal_to_world(&local.sp.n),
                u: local.sp.u,
                v: local.sp.v,
            },
            wi: self.to_world.transform_direction_to_world(&local.wi),
            pdf: local.pdf,
        })
    }
}

impl Intersectable for Transformed {
    /// Every hit the child reports, each one placed in the world.
    ///
    /// The list keeps the child's order, hence its sort by distance: a placement that preserves
    /// distances along the ray — the precondition of this module — cannot reorder them.
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        let local_ray = self.to_world.transform_ray_to_local(ray);
        let mut hits = self.shape.intersect(&local_ray, near, far);

        // The child hands back an owned vector, so each world-space copy replaces the local one
        // where it stands. Collecting into a second `IntersectionResult` would allocate once per
        // hit, on a path every ray of every render walks.
        for hit in hits.iter_mut() {
            *hit = self.to_world.transform_interaction_to_world(hit);
        }

        hits
    }

    /// Moves the ray into local space and asks the question there.
    ///
    /// A boolean needs no journey back: `intersect` above places every hit it found, and that work
    /// exists only to position points and normals a shadow ray will never read.
    fn intersect_p(&self, ray: &Ray, near: f64, far: f64) -> bool {
        let local_ray = self.to_world.transform_ray_to_local(ray);
        self.shape.intersect_p(&local_ray, near, far)
    }

    /// Whether the placed solid encloses the world-space `point`.
    ///
    /// The point travels the way a ray does, world → local, and the child answers about its own
    /// space. A shape that encloses nothing keeps enclosing nothing wherever it is put.
    fn contain_point(&self, point: &Vector3f) -> bool {
        let local_point = self.to_world.transform_point_to_local(point);
        self.shape.contain_point(&local_point)
    }
}

impl AABound for Transformed {
    /// The child's bound, placed — that is, the smallest axis-aligned box containing its eight
    /// transformed corners.
    ///
    /// Loose in general: a rotated box gets a bound larger than the geometry it holds, since an
    /// axis-aligned box cannot follow a shape that no longer lies along the axes. Tightening it
    /// would mean asking the child for a bound in a frame of our choosing, which no `AABound` can
    /// do — and a bound may be loose, never too small.
    fn get_bounding_box(&self) -> AABoundingBox {
        self.shape.get_bounding_box().transform(&self.to_world)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::geom::vector3;
    use crate::shapes::{Plane, Rectangle, Sphere};

    const EPSILON: f64 = 1e-12;

    /// A placement aligned with no axis and away from the origin, so that a conversion which
    /// silently did nothing — or applied the inverse — could not pass.
    ///
    /// Deterministic, so a test needing both the placement and a shape built with it calls this
    /// twice and gets the same placement both times.
    fn placement() -> Box<Transform> {
        let rotation = Transform::rotation_y(0.7) * Transform::rotation_x(-0.4);
        Box::new(Transform::translation(Vector3f::new(2.0, -3.0, 0.5)) * rotation)
    }

    fn assert_close(actual: &Vector3f, expected: &Vector3f, label: &str) {
        let delta = (actual - expected).length();
        assert!(delta < EPSILON, "{}: {} != {} (delta {})", label, actual, expected, delta);
    }

    /// The property an area light will rest on: a point *picked* on the surface and placed, and a
    /// point *hit* by a ray, are the same world-space point.
    ///
    /// This is the cross-check the whole decorator is for. A light samples a position on the shape
    /// and the integrator shoots a ray at it; if the two routes disagreed by a placement, the
    /// shadow ray would miss the surface it was aimed at and the error would show up only as a
    /// wrong shadow.
    ///
    /// The unit sphere makes the sampled point cheap to write down: its outward normal at `p` is
    /// `p` itself, so a surface point and its normal need no shape API to be known.
    #[test]
    fn test_a_placed_point_and_a_hit_point_agree() {
        let to_world = placement();
        let shape = Transformed::new(Arc::new(Sphere::new(1.0)), placement());

        // Sampled: a point on the child's surface, placed by hand.
        let local_p = Vector3f::new(1.0, 2.0, 3.0).normalized();
        let sampled_p = to_world.transform_point_to_world(&local_p);
        let sampled_n = to_world.transform_normal_to_world(&local_p);

        // Hit: a ray aimed at that world point from along its own normal.
        const DISTANCE: f64 = 2.5;
        let origin = sampled_p + sampled_n * DISTANCE;
        let ray = Ray::spawn_from_through(&origin, &sampled_p);

        let hits = shape.intersect(&ray, 0.0, 1000.0);
        assert!(!hits.is_empty(), "the ray must reach the placed sphere");

        assert_close(&hits[0].p, &sampled_p, "hit position");
        assert_close(&hits[0].n, &sampled_n, "hit normal");
        assert!((hits[0].d - DISTANCE).abs() < EPSILON, "hit distance {} is not {}", hits[0].d, DISTANCE);
    }

    /// Placing the geometry does not change how many times a ray crosses it, nor the order the
    /// crossings come in — and every one of them lands on the placed surface.
    #[test]
    fn test_every_hit_of_a_traversing_ray_is_placed() {
        let to_world = placement();
        let center = to_world.transform_point_to_world(&Vector3f::zero());
        let shape = Transformed::new(Arc::new(Sphere::new(1.0)), placement());

        // Straight through the placed sphere, from a direction related to no axis of either frame.
        let origin = center + Vector3f::new(-4.0, 1.5, 3.0);
        let ray = Ray::spawn_from_through(&origin, &center);

        let hits = shape.intersect(&ray, 0.0, 1000.0);
        assert_eq!(hits.len(), 2, "a ray through the centre enters and leaves");
        assert!(hits[0].d < hits[1].d, "hits must stay sorted by distance");

        for hit in hits.iter() {
            let radius = (hit.p - center).length();
            assert!((radius - 1.0).abs() < EPSILON, "hit at {} is {} from the placed centre", hit.p, radius);
            assert_close(&hit.p, &ray.point_at(hit.d), "position against its own distance");
        }
    }

    /// Nesting two placements composes them, which is what lets a shape be assembled in a frame of
    /// its own and then put somewhere — a CSG built around its own origin, placed once.
    ///
    /// It is also the guard against a *double* placement: were the outer decorator to apply its
    /// transformation twice, or the inner one to leak into the outer's frame, this comparison
    /// against the single composed placement would part company.
    #[test]
    fn test_nesting_two_placements_composes_them() {
        // Where the child sits inside the assembly, and where the assembly sits in the world.
        let inner = || Transform::translation(Vector3f::new(0.0, 1.0, 0.0));
        let outer = || Transform::rotation_z(1.1) * Transform::translation(Vector3f::new(-2.0, 0.0, 4.0));

        let nested = Transformed::new(
            Arc::new(Transformed::new(Arc::new(Sphere::new(0.5)), Box::new(inner()))),
            Box::new(outer()),
        );
        // p_world = outer ⋅ (inner ⋅ p_local), so the single placement is their product in that
        // order.
        let composed = Transformed::new(Arc::new(Sphere::new(0.5)), Box::new(&outer() * &inner()));

        let center = (&outer() * &inner()).transform_point_to_world(&Vector3f::zero());
        let ray = Ray::spawn_from_through(&(center + Vector3f::new(2.0, -3.0, 1.0)), &center);

        let nested_hits = nested.intersect(&ray, 0.0, 1000.0);
        let composed_hits = composed.intersect(&ray, 0.0, 1000.0);

        assert_eq!(nested_hits.len(), composed_hits.len());
        for (nested_hit, composed_hit) in nested_hits.iter().zip(composed_hits.iter()) {
            assert_close(&nested_hit.p, &composed_hit.p, "nested against composed position");
            assert_close(&nested_hit.n, &composed_hit.n, "nested against composed normal");
        }
    }

    /// `intersect_p` takes a shortcut — no hit is placed — so it has to be checked against the
    /// answer `intersect` gives, or the shortcut could quietly answer about another sphere.
    #[test]
    fn test_intersect_p_agrees_with_intersect() {
        let to_world = placement();
        let center = to_world.transform_point_to_world(&Vector3f::zero());
        let shape = Transformed::new(Arc::new(Sphere::new(1.0)), placement());

        let rays = [
            // Through the placed sphere.
            Ray::spawn_from_through(&(center + Vector3f::new(0.0, 5.0, 0.0)), &center),
            // Aimed at where the *unplaced* sphere would be: a miss, and the one a decorator
            // applying its transformation the wrong way round would report as a hit.
            Ray::spawn_from_through(&Vector3f::new(0.0, 5.0, 0.0), &Vector3f::zero()),
            // Pointing away from the placed sphere.
            Ray::new(&(center + Vector3f::new(0.0, 5.0, 0.0)), &Vector3f::new(0.0, 1.0, 0.0)),
        ];

        for ray in rays.iter() {
            let hits = shape.intersect(ray, 0.0, 1000.0);
            assert_eq!(
                shape.intersect_p(ray, 0.0, 1000.0),
                !hits.is_empty(),
                "the two answers disagree for ray {:?}",
                ray
            );
        }
    }

    /// The interior travels with the surface: the solid is where the placement put it, and no
    /// longer where the child alone would have it.
    #[test]
    fn test_the_interior_follows_the_placement() {
        let to_world = placement();
        let shape = Transformed::new(Arc::new(Sphere::new(1.0)), placement());

        let center = to_world.transform_point_to_world(&Vector3f::zero());
        assert!(shape.contain_point(&center), "the placed centre is inside");
        assert!(!shape.contain_point(&Vector3f::zero()), "the child's own origin is not, once moved away");
    }

    /// An open surface encloses nothing wherever it is put — the placement is a change of
    /// coordinates, not a change of what the shape is.
    #[test]
    fn test_placing_an_open_surface_gives_it_no_volume() {
        let shape = Transformed::new(Arc::new(Rectangle::new(2.0, 2.0)), placement());

        let placed_corner = placement().transform_point_to_world(&Vector3f::new(0.5, 0.0, 0.5));
        assert!(!shape.contain_point(&placed_corner));
    }

    /// The bound has to hold the geometry *after* placement, since that is the only space the
    /// accelerator knows. A bound left in the child's space would put the shape in the wrong node
    /// of the tree, and the ray that should have found it would be sent elsewhere.
    #[test]
    fn test_the_bound_holds_the_placed_geometry() {
        let to_world = placement();
        let shape = Transformed::new(Arc::new(Sphere::new(1.0)), placement());

        let bbox = shape.get_bounding_box();
        let center = to_world.transform_point_to_world(&Vector3f::zero());

        // The placement is rigid and the sphere is symmetric, so the bound stays centred on the
        // placed centre. Its extent, however, is the looseness the trait documents: the eight
        // corners of the child's cube are rotated, and the smallest axis-aligned box around them
        // grows — up to the cube's own diagonal, 2√3 — where the sphere inside it never does.
        assert_close(&bbox.centroid(), &center, "bound centroid");

        let extent = bbox.bmax - bbox.bmin;
        for axis in 0..3 {
            assert!(
                extent[axis] >= 2.0 - EPSILON && extent[axis] <= 2.0 * f64::sqrt(3.0) + EPSILON,
                "bound extent {} on axis {} is outside [2, 2√3]",
                extent[axis],
                axis
            );
        }

        // And it contains the surface points a ray actually reports.
        let ray = Ray::spawn_from_through(&(center + Vector3f::new(-4.0, 1.5, 3.0)), &center);
        for hit in shape.intersect(&ray, 0.0, 1000.0).iter() {
            for axis in 0..3 {
                assert!(
                    hit.p[axis] >= bbox.bmin[axis] - EPSILON && hit.p[axis] <= bbox.bmax[axis] + EPSILON,
                    "hit {} escapes the bound on axis {}",
                    hit.p,
                    axis
                );
            }
        }
    }

    /// The decorator relays the question rather than answering it from its own default.
    ///
    /// Both ways round, because the default `None` is right for a shape with no area and wrong for
    /// a shape that has one — and a relay that answered `Some` unconditionally would be just as
    /// broken as the default, only in the other direction. `Plane` is the honest counter-example:
    /// it is unbounded, so no placement can give it an area.
    #[test]
    fn test_the_area_sampling_face_survives_a_placement() {
        let placed_rectangle: Arc<dyn Shape> = Arc::new(Transformed::new(Arc::new(Rectangle::new(2.0, 3.0)), placement()));
        let placed_plane: Arc<dyn Shape> = Arc::new(Transformed::new(Arc::new(Plane::new()), placement()));

        assert!(placed_rectangle.area_sampler().is_some(), "a placed rectangle can still be sampled");
        assert!(placed_plane.area_sampler().is_none(), "a placed plane still has no area");
    }

    /// Placing a surface does not change its area, so it does not change the density either.
    ///
    /// `Transform` offers only rotations and translations, and an isometry maps a surface onto one
    /// of the same area. Were that to stop holding — a scaling placement — the error would be
    /// silent: an area off by a factor changes how much light a source emits without changing
    /// anything one can see about its shape.
    #[test]
    fn test_a_placement_changes_neither_the_area_nor_the_density() {
        const WIDTH: f64 = 2.0;
        const HEIGHT: f64 = 3.0;

        let placed: Arc<dyn Shape> = Arc::new(Transformed::new(Arc::new(Rectangle::new(WIDTH, HEIGHT)), placement()));
        let sampler = placed.area_sampler().unwrap();

        assert!((sampler.area() - WIDTH * HEIGHT).abs() < EPSILON, "placed area is {}", sampler.area());

        let sample = sampler.sample_area(&Vector2f::new(0.3, 0.8));
        assert!((sample.pdf - 1.0 / (WIDTH * HEIGHT)).abs() < EPSILON, "placed density is {}", sample.pdf);
    }

    /// A point *drawn* on a placed surface and a point *hit* by a ray are the same point, named the
    /// same way — the counterpart of `test_a_placed_point_and_a_hit_point_agree`, for the face a
    /// light will use.
    ///
    /// This is what the whole decorator is for, and the two halves fail differently. Were the
    /// position not placed, a light would sample a surface sitting where the object is not, and
    /// only the shadow it cast would show it. Were the texture coordinates placed — they must not
    /// be, a placement does not move a point within its own parameterisation — a textured emitter
    /// would light a scene from one part of its texture while the eye saw another.
    #[test]
    fn test_a_drawn_point_on_a_placed_surface_is_the_point_a_ray_finds() {
        let rectangle = Arc::new(Rectangle::new(2.0, 3.0));
        let placed: Arc<dyn Shape> = Arc::new(Transformed::new(Arc::clone(&rectangle) as Arc<dyn Shape>, placement()));
        let sampler = Arc::clone(&placed).area_sampler().unwrap();

        for (i, j) in [(0.1, 0.2), (0.5, 0.5), (0.9, 0.75), (0.0, 0.0)] {
            let sample = sampler.sample_area(&Vector2f::new(i, j));

            // Aimed at the drawn point from along its own normal, as the placed-hit test does.
            let origin = sample.sp.p + sample.sp.n * 2.5;
            let ray = Ray::spawn_from_through(&origin, &sample.sp.p);
            let hits = placed.intersect(&ray, 0.0, 100.0);

            assert_eq!(hits.len(), 1, "a ray aimed at {} must meet the placed surface", sample.sp.p);
            let hit = hits[0];

            assert_close(&hit.p, &sample.sp.p, "drawn against hit position");
            assert_close(&hit.n, &sample.sp.n, "drawn against hit normal");
            assert!((hit.u - sample.sp.u).abs() < EPSILON, "u: hit {}, drawn {}", hit.u, sample.sp.u);
            assert!((hit.v - sample.sp.v).abs() < EPSILON, "v: hit {}, drawn {}", hit.v, sample.sp.v);
        }
    }

    /// A normal is not a direction: it needs the inverse transpose, which is what
    /// `Transform::transform_normal_to_world` applies. Under a rotation the two agree, so the
    /// property that tells them apart is the geometric one — the placed normal stays perpendicular
    /// to the placed surface.
    #[test]
    fn test_the_placed_normal_stays_perpendicular_to_the_surface() {
        let to_world = placement();
        let center = to_world.transform_point_to_world(&Vector3f::zero());
        let shape = Transformed::new(Arc::new(Sphere::new(1.0)), placement());

        let ray = Ray::spawn_from_through(&(center + Vector3f::new(-4.0, 1.5, 3.0)), &center);

        for hit in shape.intersect(&ray, 0.0, 1000.0).iter() {
            // On a sphere the radius is the normal, so perpendicularity reads as collinearity with
            // (p − centre), up to the sign of the side the ray is on.
            let radial = (hit.p - center).normalized();
            assert!(
                (hit.n.length() - 1.0).abs() < EPSILON,
                "the placed normal is not unit: {}",
                hit.n.length()
            );
            assert!(
                (vector3::dot(&hit.n, &radial).abs() - 1.0).abs() < EPSILON,
                "the placed normal {} is not radial at {}",
                hit.n,
                hit.p
            );
        }
    }
}
