use super::{Light, VisibilityTester};
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
    fn li(&self, intersection: &Intersection) -> (Spectrum, Vector3f, VisibilityTester) {
        let w_light_pos = self.t.transform_point_to_world(&Vector3f::new(0.0, 0.0, 0.0));
        let mut wi = &w_light_pos - &intersection.p;
        wi.normalize();

        let squared_dist = wi.squared_length();
        let spectrum = &self.i * (1.0 / squared_dist);

        let tester = VisibilityTester::new(&intersection.p, &w_light_pos);
        (spectrum, wi, tester)
    }
}
