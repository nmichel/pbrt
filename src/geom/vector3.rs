use std::fmt;
use std::ops::{Add, AddAssign, Mul, MulAssign, Sub};
use std::marker::Copy;

/// A 3D vector generic type.
/// 
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct Vector3<T> {
    pub x: T,
    pub y: T,
    pub z: T
}

impl<T> fmt::Display for Vector3<T>
    where T: fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{} {} {}]", self.x, self.y, self.z)
    }
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

    /// Returns the squared length of a `Vector3`.
    /// 
    pub fn squared_length(&self) -> T
        where T: Mul<Output = T> + Add<Output = T> + Copy {

        dot(self, self)
    }
}

impl Vector3<f64> {
    pub fn normalize(&mut self) -> &mut Self {
        let norm: f64 = self.squared_length();
        let inv_norm = 1.0 / norm.sqrt();
        (*self) *= inv_norm;
        self
    } 
}

impl<T> Add for Vector3<T>
    where T: Add<Output = T> {

    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self::Output::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl<T> Add for &Vector3<T>
    where T: Add<Output = T> + Copy {

    type Output = Vector3<T>;

    fn add(self, other: Self) -> Self::Output {
        Self::Output::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

impl<T> Sub for Vector3<T>
    where T: Sub<Output = T> {

    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self::Output::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl<T> Sub for &Vector3<T>
    where T: Sub<Output = T> + Copy {

    type Output = Vector3<T>;

    fn sub(self, other: Self) -> Self::Output {
        Self::Output::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
}

impl<T> Mul<T> for Vector3<T>
    where T: Mul<Output = T> + Copy {

    type Output = Vector3<T>;

    fn mul(self, other: T) -> Self::Output {
        Self::Output::new(self.x * other, self.y * other, self.z * other)
    }
}

impl<T> Mul<T> for &Vector3<T>
    where T: Mul<Output = T> + Copy {

    type Output = Vector3<T>;

    fn mul(self, other: T) -> Self::Output {
        Self::Output::new(self.x * other, self.y * other, self.z * other)
    }
}

impl<T> AddAssign for Vector3<T>
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

impl<T> MulAssign<T> for Vector3<T>
    where T: MulAssign + Copy {

    fn mul_assign(&mut self, rhs: T) {
        self.x *= rhs;
        self.y *= rhs;
        self.z *= rhs;
    }
}

impl<T> Vector3<T>
    where T: MulAssign + Copy {

    pub fn mul_to_me(self: &mut Self, v: T) -> &mut Self {
        self.x *= v;
        self.y *= v;
        self.z *= v;
        self
    }
}

impl Vector3<f64> {
    pub fn quite_same(self: &Self, v: &Self) -> bool {
        (self - v).squared_length() < 0.0001
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
    fn test_normalize_method() {
        macro_rules! test_axis {
            ($ident: ident, $expr: expr) => {
                let mut $ident = $expr;
                $ident.normalize();
                assert!($ident.squared_length() >= 0.9999) ;
                assert!($ident.squared_length() <= 1.0001) ;
            };
        }
        test_axis!(ux, Vector3::new(1.0, 0.0, 0.0));
        test_axis!(uy, Vector3::new(0.0, 1.0, 0.0));
        test_axis!(uz, Vector3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn test_mul_to_me_method() {
        let mut a = Vector3::new(1.0, 2.0, 3.0);
        let fb = 2.0;
        let fc = -1.5;
        a
            .mul_to_me(fb)
            .mul_to_me(fc);
        assert_eq!(Vector3::new(-3.0, -6.0, -9.0), a);
    }

    #[test]
    fn test_mul_assign_method() {
        let mut a = Vector3::new(1.0, 2.0, 3.0);
        a *= 2.0;
        assert_eq!(Vector3::new(2.0, 4.0, 6.0), a);
        a *= 1.0/(2.0);
        assert_eq!(Vector3::new(1.0, 2.0, 3.0), a);
    }

    #[test]
    fn test_squared_length_method() {
        let a = Vector3::new(1.0, 2.0, 3.0);
        assert_eq!(14.0, a.squared_length());
    }

    #[test]
    fn test_dot_function() {
        let a = Vector3::new(1.0, 2.0, 3.0);
        let b = Vector3::new(3.0, 2.0, 1.0);
        assert_eq!(10.0, dot(&a, &b));
    }

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
        assert_eq!(Vector3::new(4.0, 4.0, 4.0), &a + c);
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
    fn test_add_to_me_method() {
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
