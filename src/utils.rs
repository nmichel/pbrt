use crate::geom::vector2::Vector2f;
use crate::geom::vector3::Vector3f;
use rand::distributions::{IndependentSample, Range};

pub fn random_double() -> f64 {
    rand::random::<f64>()
}

pub fn random_in_unit_disk() -> Vector2f {
    let one = Vector2f::new(1.0, 1.0);
    loop {
        let p = Vector2f::new(random_double(), random_double()) * 2.0 - one;
        if p.squared_length() < 1.0 {
            return p;
        }
    }
}

pub fn random_in_unit_sphere() -> Vector3f {
    let one = Vector3f::new(1.0, 1.0, 1.0);
    loop {
        let p = Vector3f::new(random_double(), random_double(), random_double()) * 2.0 - one;
        if p.squared_length() < 1.0 {
            return p;
        }
    }
}

pub fn random_unit_vector() -> Vector3f {
    let mut v = random_in_unit_sphere();
    v.normalize();
    v
}
