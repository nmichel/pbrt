use super::vector3::Vector3f;

pub struct Ray {
    pub origin: Vector3f,
    pub direction: Vector3f
}

impl Ray {
    pub fn new(origin: &Vector3f, direction: &Vector3f) -> Self {
        Self {
            origin : *origin,
            direction: *direction
        }
    }

    pub fn spawn_from_through(from: &Vector3f, through: &Vector3f) -> Self {
        let mut direction = through - from;
        direction.normalize();
        Self::new(from, &direction)
    }
}
