use crate::bvh::{Accumulator, BVHNode};
use crate::geom::intersectable;

use super::geom::intersectable::Intersection;
use super::geom::ray::Ray;
use super::interaction::Interaction;
use super::light::Light;
use super::primitives::Primitive;
use std::sync::Arc;

pub struct Scene {
    pub primitives: Vec<Arc<Primitive>>,
    pub lights: Vec<Arc<dyn Light>>,
    pub bvh: Option<BVHNode<Primitive>>,
}

impl Scene {
    pub fn new() -> Scene {
        // Scene { primitives: Vec::new(), lights: Arc::new(Vec::new()) }
        Scene {
            primitives: Vec::new(),
            lights: Vec::new(),
            bvh: None,
        }
    }

    pub fn add_object(&mut self, object: Arc<Primitive>) -> &mut Self {
        self.primitives.push(Arc::clone(&object));
        self
    }

    pub fn add_light(&mut self, light: Arc<dyn Light>) -> &mut Self {
        // Arc::get_mut(&mut self.lights).unwrap().push(light);
        self.lights.push(light);
        self
    }

    pub fn commit(&mut self) -> &mut Self {
        let bvh = Self::build_bvh(&mut self.primitives);
        self.primitives = vec![];
        self.bvh = bvh;
        self
    }

    fn build_bvh(prim: &mut Vec<Arc<Primitive>>) -> Option<BVHNode<Primitive>> {
        Some(BVHNode::new(prim))
    }
}

struct PrimitiveAccumulator {
    pub acc: Option<Interaction>,
}

impl Accumulator<Primitive> for PrimitiveAccumulator {
    fn accumulate(&mut self, item: &Arc<Primitive>, intersections: Vec<intersectable::Intersection>) -> () {
        let slice: &[Intersection] = intersections.as_slice();
        match slice {
            [intersection, ..] => {
                match &mut self.acc {
                    None => {
                        let inter = *intersection;
                        let material = item.material.clone();
                        self.acc = Some(Interaction {
                            intersection: inter,
                            material,
                        })
                    }
                    Some(ref prev_interaction) => {
                        if intersection.d < prev_interaction.intersection.d {
                            let material = item.material.clone();
                            self.acc = Some(Interaction {
                                intersection: *intersection,
                                material,
                            })
                        }
                        else {
                            ()
                        }
                    }
                }
            }
            _ => (),
        }
        ()
    }
}

impl Scene {
    pub fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Interaction> {
        let mut accumulator = PrimitiveAccumulator { acc: None };

        match &self.bvh {
            None => None,
            Some(bvh_node) => {
                bvh_node.intersect(ray, near, far, &mut accumulator);
                accumulator.acc
            }
        }
    }
}
