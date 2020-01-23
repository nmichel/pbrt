use super::geom::intersectable::Intersection;
use super::geom::ray::Ray;
use super::geom::transform::Transform;
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
            Some(interaction) => {
                let l = (&self.to - &self.from).squared_length();
                if (interaction.intersection.d * interaction.intersection.d) <= l { false } else { true }
            },
            None => true
        }
    }
}

pub trait Light : Send + Sync {
    fn le(&self, _ray: &Ray) -> Spectrum {
        Spectrum::new(0.0, 0.0, 0.0)
    }

    fn li(&self, _intersection: &Intersection) -> (Spectrum, Vector3f, VisibilityTester);
}

pub struct PointLight {
    t: Box<Transform>,
    i: Spectrum 
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
