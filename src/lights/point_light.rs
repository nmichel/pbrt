use super::{Light, LightLiSample, LightType, VisibilityTester};
use crate::geom::intersectable::Intersection;
use crate::geom::transform::Transform;
use crate::geom::vector3::Vector3f;
use crate::spectrum::Spectrum;

pub struct PointLight {
    t: Box<Transform>,
    i: Spectrum,
}

impl PointLight {
    pub fn new(t: Box<Transform>, i: Spectrum) -> Self {
        PointLight { t, i }
    }
}

impl Light for PointLight {
    fn light_type(&self) -> LightType {
        LightType::Point
    }

    fn sample_li(&self, intersection: &Intersection) -> Option<(LightLiSample, VisibilityTester)> {
        let world_light_pos = self.t.transform_point_to_world(&Vector3f::new(0.0, 0.0, 0.0));
        let wi = &world_light_pos - &intersection.p;
        let spectrum = &self.i / wi.squared_length();

        let sample = LightLiSample {
            spectrum: spectrum,
            wi: wi.normalized(),
            pdf: 1.0,
        };
        let tester = VisibilityTester::between(&intersection.p, &world_light_pos);
        Some((sample, tester))
    }
}
