use crate::lights::LightType;
use crate::objects::Object;

use super::bvh::{Accumulator, BVHNode};
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

    pub fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Interaction> {
        let mut accumulator = ObjectAccumulator { acc: Vec::new() };

        match &self.bvh {
            None => None,
            Some(bvh_node) => {
                // Ask the acceleration structure for potentially hit items
                bvh_node.query(ray, near, far, &mut accumulator);

                // Iterate through candidates to find the nearest interaction
                accumulator.acc.iter().fold(Option::<Interaction>::None, |acc, primitive| {
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
