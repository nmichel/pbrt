use crate::geom::intersectable;

use super::geom::intersectable::Intersectable;
use super::interaction::Interaction;
use super::geom::ray::Ray;
use super::light::Light;
use super::materials::Material;
use super::primitives::Primitive;
use std::sync::Arc;

pub struct Scene {
    pub primitives: Vec<Arc<Primitive>>,
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

    pub fn add_light(&mut self, light: Arc<dyn Light>) -> &mut Self {
        // Arc::get_mut(&mut self.lights).unwrap().push(light);
        self.lights.push(light);
        self
    }
}

impl Scene {
    pub fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Interaction> {
        let res: Option<Interaction> = None;
        self.primitives.iter().fold(res, |acc, primitive| {
            let res = primitive.intersect(ray, near, far);
            // println!("Collisions {:?}\n", &res);
            let slice: &[intersectable::Intersection] = res.as_slice();
            match slice {
                [intersection, ..] => {
                    match acc {
                        None => {
                            let inter = *intersection;
                            let material: &dyn Material = &*(primitive.material);
                            Some(Interaction { intersection: inter, material })
                        },
                        Some(ref prev_interaction) => {
                            if intersection.d < prev_interaction.intersection.d {
                                let material: &dyn Material = &*(primitive.material);
                                Some(Interaction { intersection: *intersection, material })
                            }
                            else {
                                acc
                            }
                        }
                    }
                },
                _ =>
                    acc
            }
        })
    }
}
