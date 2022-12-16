use std::marker::Copy;
use std::ops::{Add, AddAssign, Mul, Sub};

/// A 2D vector generic type.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector2<T> {
    pub x: T,
    pub y: T,
}

pub type Vector2u = Vector2<u32>;
pub type Vector2f = Vector2<f64>;

impl<T> Vector2<T> {
    /// Constructs a new `Vector2` initialized from parameters.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pbrt::geom::vector2::Vector2;
    /// let v = Vector2::new(1, 2);
    /// assert_eq!(v.x, 1);
    /// assert_eq!(v.y, 2);
    /// ```
    pub fn new(x: T, y: T) -> Self {
        Vector2 { x, y }
    }

    /// Returns the squared length of a `Vector2`.
    ///
    pub fn squared_length(&self) -> T
    where
        T: Mul<Output = T> + Add<Output = T> + Copy,
    {
        dot(self, self)
    }
}

impl<T: Add<Output = T>> Add for Vector2<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl<T> Sub for Vector2<T>
where
    T: Sub<Output = T>,
{
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self::Output::new(self.x - other.x, self.y - other.y)
    }
}

impl<T> Mul<T> for Vector2<T>
where
    T: Mul<Output = T> + Copy,
{
    type Output = Vector2<T>;

    fn mul(self, other: T) -> Self::Output {
        Self::Output::new(self.x * other, self.y * other)
    }
}

impl<T> AddAssign for Vector2<T>
where
    T: Add<Output = T> + AddAssign,
{
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl<T> AddAssign<&Vector2<T>> for Vector2<T>
where
    T: AddAssign + Copy,
{
    fn add_assign(&mut self, rhs: &Vector2<T>) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

pub fn add<T>(u: &Vector2<T>, v: &Vector2<T>) -> Vector2<T>
where
    T: Add<Output = T> + Copy,
{
    Vector2 { x: u.x + v.x, y: u.y + v.y }
}

pub fn dot<T>(u: &Vector2<T>, v: &Vector2<T>) -> T
where
    T: Mul<Output = T> + Add<Output = T> + Copy,
{
    u.x * v.x + u.y * v.y
}

impl<T> Vector2<T>
where
    T: AddAssign + Copy,
{
    pub fn add_to_me(self: &mut Self, v: &Self) -> &mut Self {
        // self += v;
        self.x += v.x;
        self.y += v.y;
        self
    }
}

impl From<Vector2u> for Vector2f {
    fn from(v: Vector2u) -> Vector2f {
        Vector2f {
            x: v.x as f64,
            y: v.y as f64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        assert_eq!(Vector2::new(1, 2), Vector2 { x: 1, y: 2 })
    }

    #[test]
    fn test_add_function() {
        let a = Vector2::new(1.0, 2.0);
        let b = Vector2::new(2.0, 1.0);
        assert_eq!(Vector2::new(3.0, 3.0), add(&a, &b));
    }

    #[test]
    fn test_add_method() {
        let a = Vector2::new(1.0, 2.0);
        let b = Vector2::new(2.0, 1.0);
        assert_eq!(Vector2::new(3.0, 3.0), a + b);
    }

    #[test]
    fn test_add_assign_rhs_ref_method() {
        let mut a = Vector2::new(1.0, 2.0);
        let b = Vector2::new(2.0, 1.0);
        a += &b;
        assert_eq!(Vector2::new(3.0, 3.0), a);

        assert_eq!(b, b);
    }

    #[test]
    fn test_add_assign_method() {
        let mut a = Vector2::new(1.0, 2.0);
        let b = Vector2::new(2.0, 1.0);
        a += b;
        assert_eq!(Vector2::new(3.0, 3.0), a);
    }

    #[test]
    fn test_add_to_method() {
        let mut a = Vector2::new(1.0, 2.0);
        let b = Vector2::new(2.0, 1.0);
        let c = Vector2::new(-2.0, -1.0);
        a.add_to_me(&b).add_to_me(&c);
        assert_eq!(Vector2::new(1.0, 2.0), a);

        assert_eq!(b, b);
        assert_eq!(c, c);
    }
}
