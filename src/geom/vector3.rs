use std::ops::{Add, AddAssign, Mul, Sub};
use std::marker::Copy;

/// A 3D vector generic type.
/// 
#[derive(Debug, PartialEq)]
pub struct Vector3<T> {
    pub x: T,
    pub y: T,
    pub z: T
}

pub type Vector3f = Vector3<f64>;

impl<T> Vector3<T> {
    /// Constructs a new `Vector3` initialized from parameters.
    /// 
    /// # Examples
    /// 
    /// ```
    /// # use pbrt::geom::vector3::Vector3;
    /// let v = Vector3::new(1, 2, 3);
    /// assert_eq!(v.x, 1);
    /// assert_eq!(v.y, 2);
    /// assert_eq!(v.z, 3);
    /// ```
    pub fn new(x: T, y: T, z: T) -> Self {
        Vector3 { x, y, z }
    }
}

impl <T> Add for Vector3<T>
    where T: Add<Output = T> {

    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl <T> Add<&Vector3<T>> for Vector3<T>
    where T: Add<Output = T> + Copy {

    type Output = Self;

    fn add(self, rhs: &Vector3<T>) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl <T> AddAssign for Vector3<T>
    where T: Add<Output = T> + AddAssign {

    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
        self.z += other.z;
    }
}

impl<T> AddAssign<&Vector3<T>> for Vector3<T>
    where T: AddAssign + Copy {

    fn add_assign(&mut self, rhs: &Vector3<T>) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl<T> Vector3<T>
    where T: AddAssign + Copy {

    pub fn add_to_me(self: &mut Self, v: &Self) -> &mut Self {
        self.x += v.x;
        self.y += v.y;
        self.z += v.z;
        self
    }
}

pub fn add<T>(u: &Vector3<T>, v: &Vector3<T>) -> Vector3<T> 
    where T: Add<Output = T> + Copy {

    Vector3 { x: u.x + v.x, y: u.y + v.y, z: u.z + v.z }
}

pub fn dot<T>(u: &Vector3<T>, v: &Vector3<T>) -> T
    where T: Mul<Output = T> + Add<Output = T> + Copy {

    u.x * v.x + u.y * v.y + u.z * v.z
}

pub fn cross<T>(u: &Vector3<T>, v: &Vector3<T>) -> Vector3<T>
    where T: Mul<Output = T> + Sub<Output = T> + Copy {

    let x = u.y * v.z - u.z * v.y;
    let y = u.z * v.x - u.x * v.z;
    let z = u.x * v.y - u.y * v.x;

    Vector3 { x, y, z }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        assert_eq!(Vector3::new(1, 2, 3), Vector3 { x: 1, y: 2, z: 3 })
    }

    #[test]
    fn test_add_function() {
        let a = Vector3::new(1.0, 2.0, 3.0);
        let b = Vector3::new(3.0, 2.0, 1.0);
        assert_eq!(Vector3::new(4.0, 4.0, 4.0), add(&a, &b));
    }

    #[test]
    fn test_add_method() {
        let a = Vector3::new(1.0, 2.0, 3.0);
        let b = Vector3::new(3.0, 2.0, 1.0);
        assert_eq!(Vector3::new(4.0, 4.0, 4.0), a + b);
    }
    #[test]
    fn test_add_method_rhs_ref() {
        let a = Vector3::new(1.0, 2.0, 3.0);
        let b = Vector3::new(3.0, 2.0, 1.0);
        let c = &b;
        assert_eq!(Vector3::new(4.0, 4.0, 4.0), a + c);
    }

    #[test]
    fn test_add_assign_method() {
        let mut a = Vector3::new(1.0, 2.0, 3.0);
        let b = Vector3::new(3.0, 2.0, 1.0);
        a += b;
        assert_eq!(Vector3::new(4.0, 4.0, 4.0), a);
    }

    #[test]
    fn test_add_assign_method_rhs_ref() {
        let mut a = Vector3::new(1.0, 2.0, 3.0);
        let b = Vector3::new(3.0, 2.0, 1.0);
        a += &b;
        assert_eq!(Vector3::new(4.0, 4.0, 4.0), a);

        assert_eq!(b, b);
    }

    #[test]
    fn test_add_to_method() {
        let mut a = Vector3::new(1.0, 2.0, 3.0);
        let b = Vector3::new(3.0, 2.0, 1.0);
        let c = Vector3::new(-3.0, -2.0, -1.0);
        a
            .add_to_me(&b)
            .add_to_me(&c);
        assert_eq!(Vector3::new(1.0, 2.0, 3.0), a);

        assert_eq!(b, b);
        assert_eq!(c, c);
    }
}
