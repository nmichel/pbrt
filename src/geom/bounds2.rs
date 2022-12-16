use super::vector2::Vector2;
use num_traits::{Num, NumAssign};
use std::iter::Iterator;

pub struct Bounds2<T>
where
    T: Num,
{
    pub min: Vector2<T>,
    pub max: Vector2<T>,
}

impl<T> Bounds2<T>
where
    T: Num + Copy,
{
    pub fn new(min: &Vector2<T>, max: &Vector2<T>) -> Self {
        // TODO: Ensure min <= max
        Self { min: *min, max: *max }
    }
}

pub struct Bounds2Iterator<'a, T>
where
    T: Num,
{
    pub p: Vector2<T>,
    pub bounds: &'a Bounds2<T>,
}

impl<'a, T> Bounds2Iterator<'a, T>
where
    T: Num + Copy,
{
    pub fn new(bounds: &'a Bounds2<T>) -> Self {
        let p = bounds.min;
        Self { p, bounds }
    }
}

impl<'a, T> Iterator for Bounds2Iterator<'a, T>
where
    T: NumAssign + Copy,
{
    type Item = Vector2<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.p.y == self.bounds.max.y {
            return None;
        }

        let curr = self.p;
        self.p.x += T::one();
        if self.p.x == self.bounds.max.x {
            self.p.x = self.bounds.min.x;
            self.p.y += T::one();
        }
        Option::Some(curr)
    }
}

impl<'a, T> Bounds2<T>
where
    T: Num + Copy,
{
    pub fn to_iter(&'a self) -> Bounds2Iterator<'a, T> {
        Bounds2Iterator::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::vector2::Vector2u;

    #[test]
    fn test_iterate() {
        let bounds2 = Bounds2::new(&Vector2u::new(0, 0), &Vector2u::new(2, 4));
        let res: Vec<_> = bounds2.to_iter().collect();
        let reference = vec![
            Vector2 { x: 0, y: 0 },
            Vector2 { x: 1, y: 0 },
            Vector2 { x: 0, y: 1 },
            Vector2 { x: 1, y: 1 },
            Vector2 { x: 0, y: 2 },
            Vector2 { x: 1, y: 2 },
            Vector2 { x: 0, y: 3 },
            Vector2 { x: 1, y: 3 },
        ];
        assert_eq!(reference, res);
    }
}
