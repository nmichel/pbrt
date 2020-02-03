use super::geom::intersectable::{Intersectable, Intersection};
use super::interaction::Interaction;
use super::geom::ray::Ray;
use super::light::Light;
use super::materials::Material;
use super::primitives::Primitive;
use super::spectrum::Spectrum;
use std::sync::Arc;

pub struct Scene {
    pub primitives: Vec<Arc<Primitive>>,
    // pub lights: Arc<Vec<Arc<dyn Light>>>
    pub lights: Vec<Arc<dyn Light>>
}

impl Scene {
    pub fn new() -> Scene {
        // Scene { primitives: Vec::new(), lights: Arc::new(Vec::new()) }
        Scene { primitives: Vec::new(), lights: Vec::new() }
    }

    pub fn add_object(&mut self, object: Arc<Primitive>) -> &mut Self {
        self.primitives.push(Arc::clone(&object));
        self
    }

    pub fn add_light(&mut self, light: Arc<Light>) -> &mut Self {
        // Arc::get_mut(&mut self.lights).unwrap().push(light);
        self.lights.push(light);
        self
    }
}

impl Scene {
    pub fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Interaction> {
        let res: Option<Interaction> = None;
        self.primitives.iter().fold(res, |acc, primitive| {
            match primitive.intersect(ray, near, far) {
                Some(intersection) => {
                    match acc {
                        None => {
                            let material: &Material = &*(primitive.material);
                            Some(Interaction { intersection, material })
                        },
                        Some(prev_interaction) => {
                            if intersection.d < prev_interaction.intersection.d {
                                let material: &Material = &*(primitive.material);
                                Some(Interaction { intersection, material })
                            }
                            else {
                                Some(prev_interaction)
                            }
                        }
                    }
                },
                None =>
                    acc
            }
        })
    }
}
