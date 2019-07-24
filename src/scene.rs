use std::cell::RefCell;
use super::geom::intersectable::{Intersectable, Intersection};
use super::geom::ray::Ray;


pub struct Scene {
    objects: Vec<Box<dyn Intersectable>>
}

impl Scene {
    pub fn new() -> Scene {
        Scene { objects: Vec::new() }
    }

    pub fn add(&mut self, object: Box<Intersectable>) -> &mut Self {
        self.objects.push(object);
        self
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
