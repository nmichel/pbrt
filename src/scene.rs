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
    fn intersect(&self, ray: &Ray) -> Vec<Intersection> {
        let res: RefCell<Vec<Intersection>> = RefCell::new(Vec::new());
        let mut intersections = 
            self.objects.iter().fold(res, |acc, item| {
                acc.borrow_mut().append(&mut item.intersect(ray));
                acc
            })
            .into_inner();

        intersections.sort_by(|a, b| a.d.partial_cmp(&b.d).unwrap());
        intersections
    }
}
