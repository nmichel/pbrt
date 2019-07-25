use super::geom::intersectable::{Intersectable, Intersection};
use super::geom::ray::Ray;
use super::geom::vector3::Vector3f;
use super::scene::Scene;
use super::spectrum::Spectrum;

pub struct VisibilityTester {
    from: Vector3f,
    to: Vector3f
}

impl VisibilityTester {
    pub fn new(from: &Vector3f, to: &Vector3f) -> Self {
        Self { from: *from, to: *to }
    }

    pub fn unoccluded(&self, scene: &Scene) -> bool {
        let ray = Ray::spawn_from_through(&self.from, &self.to);
        match scene.intersect(&ray) {
            Some(_) => false,
            None => true
        }
    }
}

pub trait Light {
    fn le(&self, _ray: &Ray) -> Spectrum {
        Spectrum::new(0.0, 0.0, 0.0)
    }

    fn li(&self, _intersection: &Intersection) -> (Spectrum, Vector3f, VisibilityTester);
}

pub struct PointLight {
    p: Vector3f,
    i: Spectrum 
}

impl PointLight {
    pub fn new(p: Vector3f, i: Spectrum) -> Self {
        PointLight { p, i }
    }
}

impl Light for PointLight {
    fn li(&self, intersection: &Intersection) -> (Spectrum, Vector3f, VisibilityTester) {
        let mut wi = &self.p - &intersection.p;
        wi.normalize();

        let squared_dist = wi.squared_length();
        let spectrum = &self.i * (1.0 / squared_dist);

        let tester = VisibilityTester::new(&intersection.p, &self.p);
        (spectrum, wi, tester)
    }
}
