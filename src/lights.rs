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

#[derive(Debug, PartialEq)]
pub enum LightType {
    Point,
    Infinite,
}

/// This struct captures the result of sampling a light source at a given shading point.
pub struct LightLiSample {
    /// The spectral radiance received from the sampled point on the light source.
    pub spectrum: Spectrum,

    /// The direction from the shading point to the sampled point on the light source.
    pub wi: Vector3f,

    /// The probability density function value associated with sampling `wi`.
    pub pdf: f64,
}

pub trait Light: Send + Sync {
    /// Returns the type of light
    ///
    /// Depending on the context some light types may or may not be used.
    fn light_type(&self) -> LightType;

    fn le(&self, _ray: &Ray) -> Spectrum {
        Spectrum::new(0.0, 0.0, 0.0)
    }

    fn sample_li(&self, _intersection: &Intersection) -> Option<(LightLiSample, VisibilityTester)>;
}

mod background_infinite_light;
mod point_light;
mod uniform_infinite_light;

pub use self::background_infinite_light::BackgroundInfiniteLight;
pub use self::point_light::PointLight;
pub use self::uniform_infinite_light::UniformInfiniteLight;
