use super::bvh::BVHNode;
use super::geom::ray::Ray;
use super::interaction::Interaction;
use super::light::Light;
use super::primitives::Primitive;
use std::sync::Arc;

pub struct Scene {
    pub primitives: Vec<Arc<Primitive>>,
    pub lights: Vec<Arc<dyn Light>>,
    pub bvh: Option<BVHNode>,
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

    fn build_bvh(prim: &mut Vec<Arc<Primitive>>) -> Option<BVHNode> {
        Some(BVHNode::new(prim))
    }
}

impl Scene {
    pub fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Interaction> {
        match &self.bvh {
            None => None,
            Some(bvh_node) => bvh_node.intersect(ray, near, far),
        }
    }
}
