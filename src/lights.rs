use super::geom::intersectable::Intersection;
use super::geom::ray::Ray;
use super::geom::vector3::Vector3f;
use super::scene::Scene;
use super::spectrum::Spectrum;

pub struct VisibilityTester {
    from: Vector3f,
    to: Vector3f,
}

impl VisibilityTester {
    pub fn new(from: &Vector3f, to: &Vector3f) -> Self {
        Self { from: *from, to: *to }
    }

    pub fn unoccluded(&self, scene: &Scene) -> bool {
        let ray = Ray::spawn_from_through(&self.from, &self.to);
        match scene.intersect(&ray, 0.00001, std::f64::MAX) {
            Some(interaction) => {
                let l = (&self.to - &self.from).squared_length();
                if (interaction.intersection.d * interaction.intersection.d) <= l {
                    false
                }
                else {
                    true
                }
            }
            None => true,
        }
    }
}

pub trait Light: Send + Sync {
    fn le(&self, _ray: &Ray) -> Spectrum {
        Spectrum::new(0.0, 0.0, 0.0)
    }

    fn li(&self, _intersection: &Intersection) -> (Spectrum, Vector3f, VisibilityTester);
}

mod point_light;

pub use point_light::PointLight;
