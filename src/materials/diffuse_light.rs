//! A surface that emits, uniformly over the hemisphere it faces.
//!
//! Reference: PBR Book, 4ed, §12.4 — *Light Sources*, « Area Lights ».
//! <https://pbr-book.org/4ed/Light_Sources/Area_Lights>

use super::{Emitter, Material};
use crate::colors;
use crate::geom::ray::Ray;
use crate::geom::surface_point::SurfacePoint;
use crate::geom::vector3;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::spectrum::Spectrum;
use crate::textures::*;
use std::sync::Arc;

pub struct DiffuseLight {
    emitted: Arc<dyn Texture>,
}

impl DiffuseLight {
    pub fn new(emitted: Arc<dyn Texture>) -> Self {
        Self { emitted }
    }
}

impl Emitter for DiffuseLight {
    /// The radiance leaving `sp` towards `w`, zero on the face that does not emit.
    ///
    /// # Frame
    ///
    /// Both vectors are in world space. `sp.n` is the normal as the shape reports it — no shape in
    /// the project turns its normal towards the incoming ray, so it states an orientation the scene
    /// chose rather than one a ray decided. `w` points *away* from the surface, so `sp.n ⋅ w` is
    /// the cosine of the angle between the normal and the direction the light is leaving in.
    ///
    /// # The emission is one-sided
    ///
    /// Radiance is emitted into the hemisphere about `+n` and nowhere else:
    ///
    /// ```text
    /// L(w) = Lₑ   if n ⋅ w > 0
    ///      = 0    otherwise
    /// ```
    ///
    /// `Lₑ` does not vary with `w` within that hemisphere — that is what makes the emitter
    /// *diffuse*, the emitted counterpart of a Lambertian reflector.
    ///
    /// The grazing case `n ⋅ w = 0` emits nothing, which is the limit and not a convention: a
    /// surface seen exactly edge-on subtends no solid angle, so its contribution vanishes there
    /// whichever side one approaches from.
    ///
    /// # A departure: no two-sided emitter
    ///
    /// pbrt lets a `DiffuseAreaLight` be declared two-sided, for geometry that stands for a thin
    /// object lit from within — a lamp shade, a bulb modelled as a shell. Nothing here offers that
    /// choice, because no scene in the project needs it, and the `.stage` grammar would have to
    /// carry a keyword for a setting no file would set.
    ///
    /// The consequence is that **an emissive surface has to be oriented by the scene that declares
    /// it**: a panel whose normal faces the ceiling lights nothing below it. That failure is
    /// visible — the panel is simply black — rather than silent, which is the reason it is an
    /// acceptable price. Emitting on both faces would cost the opposite: a panel would light the
    /// room no matter how it was placed, and the scene would never have to say which way its lamp
    /// points.
    fn l(&self, sp: &SurfacePoint, w: &Vector3f) -> Spectrum {
        let cos_theta = vector3::dot(&sp.n, w);
        if cos_theta <= 0.0 {
            return colors::BLACK;
        }

        self.emitted.shade(sp)
    }
}

impl Material for DiffuseLight {
    /// Delegates to [`Emitter::l`], which is where the rule lives.
    ///
    /// The two answer the same question for different callers — this one for a ray that reached the
    /// surface, the trait method for anyone holding a point on it — so the one that has an
    /// `Interaction` unpacks it and asks the other. `Some` regardless of the face: a dark face
    /// emits zero, and `None` is reserved for a material that does not emit at all.
    fn emit(&self, _ray: &Ray, interaction: &Interaction) -> Option<Spectrum> {
        let surface_point = SurfacePoint::from(&interaction.intersection);

        Some(self.l(&surface_point, &interaction.intersection.wo))
    }

    fn emitter(self: Arc<Self>) -> Option<Arc<dyn Emitter>> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::colors;
    use crate::geom::intersectable::Intersection;
    use crate::geom::vector3::Vector3f;
    use crate::textures::PlainColor;

    const EMITTED: Spectrum = colors::WHITE;

    fn diffuse_light() -> DiffuseLight {
        DiffuseLight::new(Arc::new(PlainColor::new(EMITTED)))
    }

    /// A surface in the z = 0 plane facing `+z`.
    ///
    /// Only `n` bears on the answer; `p`, `u` and `v` carry values that are valid for such a
    /// surface rather than zeroes, so that the point describes a geometry that could exist.
    fn facing_up() -> SurfacePoint {
        SurfacePoint {
            p: Vector3f::zero(),
            n: Vector3f::new(0.0, 0.0, 1.0),
            u: 0.5,
            v: 0.5,
        }
    }

    /// That same surface, reached by a ray arriving from `wo` — what `Material::emit` is handed.
    fn seen_from(wo: Vector3f) -> Interaction {
        let SurfacePoint { p, n, u, v } = facing_up();

        Interaction {
            intersection: Intersection {
                p,
                d: 1.0,
                n,
                wo: wo.normalized(),
                u,
                v,
                dpdu: Vector3f::new(1.0, 0.0, 0.0),
                dpdv: Vector3f::new(0.0, 1.0, 0.0),
            },
            material: Arc::new(diffuse_light()),
        }
    }

    /// `emit` does not read the ray, and its answer must not depend on it. One ray serves every
    /// case.
    fn any_ray() -> Ray {
        Ray::new(&Vector3f::new(0.0, 0.0, 1.0), &Vector3f::new(0.0, 0.0, -1.0))
    }

    /// The three tests below ask [`Emitter::l`] and not `Material::emit`, because `l` is where the
    /// rule lives. Asked through `emit`, they would still pass if the cosine test moved back out of
    /// `l` into the delegation — which is the one drift this pair of methods exists to prevent.
    #[test]
    fn test_the_face_the_normal_points_to_emits() {
        let emitter = diffuse_light();

        // Straight on, and well off the axis but still above the surface.
        for w in [Vector3f::new(0.0, 0.0, 1.0), Vector3f::new(3.0, -4.0, 1.0)] {
            assert_eq!(emitter.l(&facing_up(), &w.normalized()), EMITTED, "w = {}", w);
        }
    }

    /// The other face is not dimmer, it is dark: a one-sided emitter seen from behind is a black
    /// surface. An integrator adds that zero like any other radiance, and adding anything else
    /// would be counting light that is not there.
    #[test]
    fn test_the_other_face_emits_nothing() {
        let emitter = diffuse_light();

        for w in [Vector3f::new(0.0, 0.0, -1.0), Vector3f::new(3.0, -4.0, -1.0)] {
            assert_eq!(emitter.l(&facing_up(), &w.normalized()), colors::BLACK, "w = {}", w);
        }
    }

    /// Edge-on is the boundary between the two, and it belongs to the dark side — the limit of a
    /// vanishing projected area, approached from either face.
    #[test]
    fn test_a_grazing_direction_emits_nothing() {
        let emitter = diffuse_light();

        assert_eq!(emitter.l(&facing_up(), &Vector3f::new(1.0, 2.0, 0.0).normalized()), colors::BLACK);
    }

    /// The material and its emitting face are one answer asked two ways, and `emitter()` is asked
    /// through an `Arc<dyn Material>` — the type a scene builder holds, and the only receiver that
    /// exercises the upcast this method exists for. Through the concrete type it would prove
    /// nothing.
    ///
    /// Both faces, because the dark one is where the two could drift apart: `emit` answering `None`
    /// there while `l` answers black would be two readings of the one-sided rule.
    #[test]
    fn test_the_material_and_its_emitter_agree_on_both_faces() {
        let material: Arc<dyn Material> = Arc::new(diffuse_light());
        let emitter = Arc::clone(&material).emitter().expect("a diffuse light is an emitter");

        for (wo, expected) in [(Vector3f::new(0.0, 0.0, 1.0), EMITTED), (Vector3f::new(0.0, 0.0, -1.0), colors::BLACK)] {
            let interaction = seen_from(wo);
            let surface_point = SurfacePoint::from(&interaction.intersection);

            assert_eq!(emitter.l(&surface_point, &interaction.intersection.wo), expected, "wo = {}", wo);
            assert_eq!(material.emit(&any_ray(), &interaction), Some(expected), "wo = {}", wo);
        }
    }
}
