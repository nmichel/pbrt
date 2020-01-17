use super::geom::intersectable::{Intersectable, Intersection};
use super::interaction::Interaction;
use super::geom::ray::Ray;
use super::light::Light;
use super::materials::material::Material;
use super::primitives::Primitive;
use super::spectrum::Spectrum;

pub struct Scene {
    pub primitives: Vec<Box<Primitive>>,
    pub lights: Vec<Box<dyn Light>>
}

impl Scene {
    pub fn new() -> Scene {
        Scene { primitives: Vec::new(), lights: Vec::new() }
    }

    pub fn add_object(&mut self, object: Box<Primitive>) -> &mut Self {
        self.primitives.push(object);
        self
    }

    pub fn add_light(&mut self, light: Box<Light>) -> &mut Self {
        self.lights.push(light);
        self
    }

    pub fn background_radiance(&self, ray: &Ray) -> Spectrum {
        // self.lights.iter().fold(Spectrum::new(0.0, 0.0, 0.0), |res, light| {
        //     res + light.le(&ray)
        // })
        Spectrum::new(0.5, 0.5, 0.0)
    }    
}

impl Scene {
    pub fn intersect(&self, ray: &Ray) -> Option<Interaction> {
        let res: Option<Interaction> = None;
        self.primitives.iter().fold(res, |acc, primitive| {
            match primitive.intersect(ray) {
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
