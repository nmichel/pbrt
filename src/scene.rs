use crate::lights::LightType;
use crate::objects::Object;

use super::bvh::{Accumulator, BVHNode, TraversalStats};
use super::geom::aabound::{AABound, AABoundingBox};
use super::geom::intersectable::{Intersectable, IntersectionResult};
use super::geom::ray::Ray;
use super::geom::vector3::Vector3f;
use super::interaction::Interaction;
use super::lights::Light;
use std::sync::Arc;

struct Wrapper<T>(Arc<T>)
where
    T: Object + ?Sized;

impl<T> Intersectable for Wrapper<T>
where
    T: Object + ?Sized,
{
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        Intersectable::intersect(self.0.as_ref(), ray, near, far)
    }

    fn contain_point(&self, point: &Vector3f) -> bool {
        self.0.contain_point(point)
    }
}

impl<T> Clone for Wrapper<T>
where
    T: Object + ?Sized,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> AABound for Wrapper<T>
where
    T: Object + ?Sized,
{
    fn get_bounding_box(&self) -> AABoundingBox {
        self.0.get_bounding_box()
    }
}

impl<T> Object for Wrapper<T>
where
    T: Object + ?Sized,
{
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Interaction> {
        Object::intersect(self.0.as_ref(), ray, near, far)
    }
}

pub struct Scene {
    primitives: Vec<Wrapper<dyn Object>>,
    lights: Vec<Arc<dyn Light>>,
    bvh: Option<BVHNode<Wrapper<dyn Object>>>,
}

impl Scene {
    pub fn new() -> Scene {
        Scene {
            primitives: Vec::new(),
            lights: Vec::new(),
            bvh: None,
        }
    }

    pub fn add_object(&mut self, object: Arc<dyn Object>) -> &mut Self {
        self.primitives.push(Wrapper(Arc::clone(&object)));
        self
    }

    pub fn add_light(&mut self, light: Arc<dyn Light>) -> &mut Self {
        self.lights.push(light);
        self
    }

    pub fn commit(&mut self) -> &mut Self {
        let bvh = Self::build_bvh(&mut self.primitives);
        self.primitives = vec![];
        self.bvh = bvh;
        self
    }

    /// Nearest interaction of `ray` with the scene, within `[near, far]`.
    pub fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Interaction> {
        self.find_nearest(ray, near, far, &mut TraversalStats::default())
    }

    /// Same as [`Scene::intersect`], but adds the work done to `stats`.
    ///
    /// Both entry points run the very same `find_nearest`, so the measured search is the one the
    /// renderer performs.
    pub fn intersect_instrumented(&self, ray: &Ray, near: f64, far: f64, stats: &mut TraversalStats) -> Option<Interaction> {
        self.find_nearest(ray, near, far, stats)
    }

    /// Whether anything at all stands along `ray` within `[near, far]`.
    ///
    /// Not `intersect(..).is_some()`. The question is different, and so is the work it deserves:
    /// *any* hit settles it, so the search stops at the first one instead of ranking every
    /// candidate by distance, and no `Interaction` is built — no material, and none of the
    /// shading frame a surface whose only role is to be in the way will never need.
    ///
    /// Deliberately unordered. Ordering exists to reach the *nearest* hit sooner; here any hit is
    /// as good as any other, and sorting children would be work spent on a distinction that does
    /// not matter.
    ///
    /// This is the query shadow rays want, and `far` is what carries the light's distance: an
    /// occluder beyond the light is not an occluder.
    pub fn intersect_p(&self, ray: &Ray, near: f64, far: f64) -> bool {
        self.any_hit(ray, near, far, &mut TraversalStats::default())
    }

    /// Same as [`Scene::intersect_p`], but adds the work done to `stats`.
    pub fn intersect_p_instrumented(&self, ray: &Ray, near: f64, far: f64, stats: &mut TraversalStats) -> bool {
        self.any_hit(ray, near, far, stats)
    }

    /// Number of primitives the accelerator was built over, or zero for an empty scene.
    pub fn primitive_count(&self) -> usize {
        self.bvh.as_ref().map_or(0, |bvh_node| bvh_node.primitive_count())
    }

    fn any_hit(&self, ray: &Ray, near: f64, far: f64, stats: &mut TraversalStats) -> bool {
        let mut accumulator = ObjectAccumulator { acc: Vec::new() };

        match &self.bvh {
            None => false,
            Some(bvh_node) => {
                bvh_node.query_instrumented(ray, near, far, &mut accumulator, stats);

                // `any` short-circuits, so candidates after the first occluder are never tested.
                // `Intersectable::intersect` rather than `Object::intersect`: geometry is the
                // whole question, and the material would only be cloned to be dropped.
                accumulator.acc.iter().any(|primitive| {
                    stats.object_tests += 1;
                    !Intersectable::intersect(primitive, ray, near, far).is_empty()
                })
            }
        }
    }

    fn find_nearest(&self, ray: &Ray, near: f64, far: f64, stats: &mut TraversalStats) -> Option<Interaction> {
        let mut accumulator = ObjectAccumulator { acc: Vec::new() };

        match &self.bvh {
            None => None,
            Some(bvh_node) => {
                // Ask the acceleration structure for potentially hit items
                bvh_node.query_instrumented(ray, near, far, &mut accumulator, stats);

                // Iterate through candidates to find the nearest interaction
                accumulator.acc.iter().fold(Option::<Interaction>::None, |acc, primitive| {
                    stats.object_tests += 1;
                    match (acc, Object::intersect(primitive, ray, near, far)) {
                        (acc, None) => acc,
                        (None, interaction) => interaction,
                        (Some(prev_interaction), Some(interaction)) => {
                            if interaction.intersection.d < prev_interaction.intersection.d {
                                Some(interaction)
                            }
                            else {
                                Some(prev_interaction)
                            }
                        }
                    }
                })
            }
        }
    }

    pub fn get_light_count(&self) -> usize {
        self.lights.len()
    }

    pub fn get_light_at<'a>(&'a self, index: usize) -> Option<&'a dyn Light> {
        if index < self.lights.len() {
            Some(self.lights[index].as_ref())
        }
        else {
            None
        }
    }

    pub fn query_lights<'a>(&'a self, light_type: &LightType) -> Vec<&'a dyn Light> {
        self.lights
            .iter()
            .filter(|light| light.light_type() == *light_type)
            .map(|light| light.as_ref())
            .collect()
    }

    /// Builds the acceleration structure over `prim`, or `None` when there is nothing to
    /// accelerate.
    ///
    /// An empty scene has no tree, and that is how emptiness is represented here: `Option`
    /// already carries it, so `BVHNode` never has to. `intersect` reads `None` as "nothing to
    /// hit" and returns without a single box test.
    fn build_bvh(prim: &mut Vec<Wrapper<dyn Object>>) -> Option<BVHNode<Wrapper<dyn Object>>> {
        if prim.is_empty() {
            return None;
        }

        Some(BVHNode::new(prim))
    }
}

struct ObjectAccumulator {
    pub acc: Vec<Wrapper<dyn Object>>,
}

impl Accumulator<Wrapper<dyn Object>> for ObjectAccumulator {
    fn accumulate(&mut self, items: &mut Vec<Wrapper<dyn Object>>) -> () {
        self.acc.append(items);
        ()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::vector3::Vector3f;

    /// A scene with no object has no tree, and `intersect` must say so.
    ///
    /// This did not use to fail, it aborted: `commit` handed an empty vector to `BVHNode::new`,
    /// whose median split left the left half empty and recursed on it until the stack overflowed.
    /// Verified by removing the guard — `fatal runtime error: stack overflow`, SIGABRT.
    #[test]
    fn test_empty_scene_intersects_nothing() {
        let mut scene = Scene::new();
        scene.commit();

        let ray = Ray::new(&Vector3f::zero(), &Vector3f::new(0.0, 0.0, 1.0));

        assert!(scene.intersect(&ray, 0.0001, 1000.0).is_none());
    }
}
