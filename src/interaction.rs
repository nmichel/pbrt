use super::geom::intersectable::Intersection;
use super::lights::Light;
use super::materials::Material;
use std::sync::Arc;

pub struct Interaction {
    /// Geometry of the interaction
    pub intersection: Intersection,

    /// Material at intersection
    pub material: Arc<dyn Material>,

    /// The source this surface *is*, when it emits and the scene samples it as a light.
    ///
    /// Reference: PBR Book, 4ed, §7.1.1 — *Geometric Primitives*, whose `SurfaceInteraction`
    /// carries the same pointer. <https://pbr-book.org/4ed/Primitives_and_Intersection_Acceleration/Primitive_Interface_and_Geometric_Primitives>
    ///
    /// A path that reaches an emitter by sampling its bsdf has found a direction that next event
    /// estimation could also have drawn, and multiple importance sampling has to ask *that* source
    /// how likely it was — [`Light::pdf_li`]. The material cannot answer: it says what the surface
    /// emits, not which entry of `Scene::lights` samples it. Nor can the integrator search the list
    /// for it, a direction reaching several sources having one density per source and only one of
    /// them being the one that was hit.
    ///
    /// `None` for every surface that is not a light — and for an emitter no source samples, which
    /// only an object built by hand can be: next event estimation never draws towards it, so the
    /// whole of its emission belongs to the path that found it.
    pub area_light: Option<Arc<dyn Light>>,
}
