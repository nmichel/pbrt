use super::geom::intersectable::{Intersectable, Intersection};
use super::geom::ray::Ray;
use super::light::Light;
use super::spectrum::Spectrum;

pub struct Scene {
    pub objects: Vec<Box<dyn Intersectable>>,
    pub lights: Vec<Box<dyn Light>>
}

impl Scene {
    pub fn new() -> Scene {
        Scene { objects: Vec::new(), lights: Vec::new() }
    }

    pub fn add_object(&mut self, object: Box<Intersectable>) -> &mut Self {
        self.objects.push(object);
        self
    }

    pub fn add_light(&mut self, light: Box<Light>) -> &mut Self {
        self.lights.push(light);
        self
    }

    pub fn background_radiance(&self, ray: &Ray) -> Spectrum {
        self.lights.iter().fold(Spectrum::new(0.0, 0.0, 0.0), |res, light| {
            res + light.le(&ray)
        })
    }
}

impl Intersectable for Scene {
    fn intersect(&self, ray: &Ray) -> Option<Intersection> {
        let res: Option<Intersection> = None;
        self.objects.iter().fold(res, |acc, item| {
            match item.intersect(ray) {
                Some(intersection) => {
                    match acc {
                        None =>
                            Some(intersection),
                        Some(prev_intersection) => {
                            if intersection.d < prev_intersection.d {
                                Some(intersection)
                            }
                            else {
                                Some(prev_intersection)
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
