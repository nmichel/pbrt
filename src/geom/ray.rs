use super::vector3::Vector3f;

pub struct Ray {
    pub origin: Vector3f,
    pub direction: Vector3f
}

impl Ray {
    pub fn new(origin: Vector3f, direction: Vector3f) -> Self {
        Self {
            origin,
            direction
        }
    }
}
