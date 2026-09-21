use std::sync::Arc;

use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::transform::{Transform, Transformable};
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;

use super::Object;

/// A built object seen from somewhere else: the ray travels into its space, the hit travels back.
///
/// # Two jobs, and which one applies where
///
/// The type is the same, the reason for reaching for it is not:
///
/// - **Programmatic placement.** `examples/` builds its scenes by hand, with no loader and
///   therefore no current transformation matrix, and this is how it puts an object somewhere. That
///   use is as correct as it ever was: there is no description being walked, so there is nothing to
///   fold a placement into.
/// - **Instancing, in the loader.** A description that can *name* a structure and repeat it wants
///   exactly this: one built object, seen from several places. It has no caller yet — naming is not
///   in the `.stage` grammar — so this is a tool waiting for one rather than dead code.
///
/// What the loader no longer does is *place* with it. A shape comes out of
/// [`SceneBuilderVisitor`](crate::loader) already in world space, folded there by
/// [`shapes::Transformed`](crate::shapes::Transformed), so wrapping the finished object would place
/// it twice.
///
/// # What instancing costs, and why placement is not the same thing
///
/// Folding a placement into geometry costs nothing per ray: the shape is simply built where it
/// belongs. Instancing cannot be free, and the difference is worth stating before someone reaches
/// for this type to place a single object:
///
/// - **one ray round trip per instance**, world → local on the way in and local → world on the way
///   out, on every ray that reaches the instance;
/// - **a sub-tree the scene's accelerator cannot merge**: the instance is one primitive to the
///   scene BVH, whatever it contains, so the tree cannot interleave its contents with the geometry
///   around it.
///
/// Both are the price of building a structure once instead of *n* times. Neither is a price worth
/// paying for a thing that appears once.
pub struct Transformed {
    pub object: Arc<dyn Object>,
    pub transform: Box<Transform>,
}

impl Transformed {
    pub fn new(object: Arc<dyn Object>, transform: Box<Transform>) -> Self {
        Self {
            object: Arc::clone(&object),
            transform: transform,
        }
    }
}

impl Object for Transformed {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Interaction> {
        let local_ray: Ray = self.transform.transform_ray_to_local(&ray);
        match Object::intersect(self.object.as_ref(), &local_ray, near, far) {
            None => None,
            Some(ref interaction) => {
                Some(Interaction {
                    intersection: self.transform.transform_interaction_to_world(&interaction.intersection),
                    material: interaction.material.clone(),
                })
            }
        }
    }
}

impl Intersectable for Transformed {
    /// Every hit the child reports, brought back into world space.
    ///
    /// Written like [`shapes::Transformed::intersect`](crate::shapes::Transformed), which is to say
    /// in place: the child hands back an owned vector, so each world-space copy replaces the local
    /// one where it stands. **No caller reaches this method** — the nearest-hit path goes through
    /// `Object::intersect` above and shadow rays through `intersect_p` below, the two of them
    /// leaving `Intersectable::intersect` alive on nothing but the `Object: Intersectable` bound
    /// (`IDEAS.md`, *Renderer & infrastructure*). So this costs nothing and saves nothing; it is
    /// written this way so that the two decorators read the same.
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        let local_ray: Ray = self.transform.transform_ray_to_local(&ray);
        let mut hits = Intersectable::intersect(self.object.as_ref(), &local_ray, near, far);

        for hit in hits.iter_mut() {
            *hit = self.transform.transform_interaction_to_world(hit);
        }

        hits
    }

    /// Moves the ray into local space and asks the question there.
    ///
    /// A boolean needs no journey back: `intersect` above has to build a second
    /// `IntersectionResult` to carry every hit into world space, and that whole round trip exists
    /// only to place points and normals a shadow ray will never read.
    ///
    /// `near` and `far` cross unchanged, exactly as they do in `intersect` — which is only sound
    /// while transforms preserve distances along the ray. A scaling transform would break both
    /// equally; this method inherits that assumption rather than adding one.
    fn intersect_p(&self, ray: &Ray, near: f64, far: f64) -> bool {
        let local_ray: Ray = self.transform.transform_ray_to_local(&ray);
        self.object.intersect_p(&local_ray, near, far)
    }

    fn contain_point(&self, point: &Vector3f) -> bool {
        let local_point = self.transform.transform_point_to_local(&point);
        self.object.contain_point(&local_point)
    }
}

impl AABound for Transformed {
    fn get_bounding_box(&self) -> AABoundingBox {
        self.object.get_bounding_box().transform(&self.transform)
    }
}
