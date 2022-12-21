use super::vector3::Vector3f;

pub struct Ray {
    pub origin: Vector3f,
    pub direction: Vector3f,
}

impl Ray {
    pub fn new(origin: &Vector3f, direction: &Vector3f) -> Self {
        let mut dir = *direction;
        dir.normalize();

        Self {
            origin: *origin,
            direction: dir,
        }
    }

    pub fn spawn_from_through(from: &Vector3f, through: &Vector3f) -> Self {
        let mut direction = through - from;
        direction.normalize();
        Self::new(from, &direction)
    }

    pub fn point_at(&self, d: f64) -> Vector3f {
        self.origin + self.direction * d
    }
}
