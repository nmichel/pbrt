//! A surface that emits, uniformly over the hemisphere it faces.
//!
//! Reference: PBR Book, 3ed, chapter 12 — *Light Sources*, « Area Lights ».
//! <https://www.pbr-book.org/3ed-2018/Light_Sources/Area_Lights>

use super::Material;
use crate::geom::ray::Ray;
use crate::geom::surface_point::SurfacePoint;
use crate::geom::vector3;
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

impl Material for DiffuseLight {
    /// The radiance leaving the surface towards `wo`, or `None` on the face that does not emit.
    ///
    /// # Frame
    ///
    /// Both vectors are in world space. `n` is the normal as the shape reports it — no shape in the
    /// project turns its normal towards the incoming ray, so `n` states an orientation the scene
    /// chose rather than one the ray decided. `wo` points *away* from the surface, back along the
    /// ray that arrived, so `n ⋅ wo` is the cosine of the angle between the normal and the
    /// direction the light is leaving in.
    ///
    /// # The emission is one-sided
    ///
    /// Radiance is emitted into the hemisphere about `+n` and nowhere else:
    ///
    /// ```text
    /// L(wo) = Lₑ   if n ⋅ wo > 0
    ///       = 0    otherwise
    /// ```
    ///
    /// `Lₑ` does not vary with `wo` within that hemisphere — that is what makes the emitter
    /// *diffuse*, the emitted counterpart of a Lambertian reflector.
    ///
    /// The grazing case `n ⋅ wo = 0` emits nothing, which is the limit and not a convention: a
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
    fn emit(&self, _ray: &Ray, interaction: &Interaction) -> Option<Spectrum> {
        let surface_point = SurfacePoint::from(&interaction.intersection);
        let cos_theta = vector3::dot(&surface_point.n, &interaction.intersection.wo);
        if cos_theta <= 0.0 {
            return None;
        }

        Some(self.emitted.shade(&surface_point))
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

    /// A surface in the z = 0 plane facing `+z`, looked at from direction `wo`.
    ///
    /// Only `n` and `wo` bear on the answer; the remaining fields are filled with values that are
    /// valid for such a surface rather than with zeroes, so that the interaction describes a
    /// geometry that could exist.
    fn seen_from(wo: Vector3f) -> Interaction {
        Interaction {
            intersection: Intersection {
                p: Vector3f::zero(),
                d: 1.0,
                n: Vector3f::new(0.0, 0.0, 1.0),
                wo: wo.normalized(),
                u: 0.5,
                v: 0.5,
                dpdu: Vector3f::new(1.0, 0.0, 0.0),
                dpdv: Vector3f::new(0.0, 1.0, 0.0),
            },
            material: Arc::new(diffuse_light()),
        }
    }

    /// The ray used to be the argument `emit` read; it is now unused, and the answer must not
    /// depend on it. One ray serves every case.
    fn any_ray() -> Ray {
        Ray::new(&Vector3f::new(0.0, 0.0, 1.0), &Vector3f::new(0.0, 0.0, -1.0))
    }

    #[test]
    fn test_the_face_the_normal_points_to_emits() {
        let material = diffuse_light();

        // Straight on, and well off the axis but still above the surface.
        for wo in [Vector3f::new(0.0, 0.0, 1.0), Vector3f::new(3.0, -4.0, 1.0)] {
            assert_eq!(material.emit(&any_ray(), &seen_from(wo)), Some(EMITTED), "wo = {}", wo);
        }
    }

    /// The other face is not dimmer, it is dark: a one-sided emitter seen from behind is a black
    /// surface, and an integrator adding anything there would be counting light that is not there.
    #[test]
    fn test_the_other_face_emits_nothing() {
        let material = diffuse_light();

        for wo in [Vector3f::new(0.0, 0.0, -1.0), Vector3f::new(3.0, -4.0, -1.0)] {
            assert_eq!(material.emit(&any_ray(), &seen_from(wo)), None, "wo = {}", wo);
        }
    }

    /// Edge-on is the boundary between the two, and it belongs to the dark side — the limit of a
    /// vanishing projected area, approached from either face.
    #[test]
    fn test_a_grazing_direction_emits_nothing() {
        let material = diffuse_light();

        assert_eq!(material.emit(&any_ray(), &seen_from(Vector3f::new(1.0, 2.0, 0.0))), None);
    }
}
