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

    fn build_bvh(prim: &mut Vec<Wrapper<dyn Object>>) -> Option<BVHNode<Wrapper<dyn Object>>> {
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
