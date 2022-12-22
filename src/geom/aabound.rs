use super::ray::Ray;
use super::transform::{Transform, Transformable};
use super::vector3::Vector3f;
use std::mem::swap;

#[derive(Debug)]
pub struct AABoundingBox {
    pub bmin: Vector3f,
    pub bmax: Vector3f,
}

/// Trait implemented by object that can be contained in a Axis Aligned Bounding Box.
/// Used by acceleration structure to effeciently search for collisions.
pub trait AABound {
    fn get_bounding_box(&self) -> AABoundingBox;
}

impl AABoundingBox {
    pub fn new(min: &Vector3f, max: &Vector3f) -> Self {
        Self { bmin: *min, bmax: *max }
    }

    /// Borrowed from https://raytracing.github.io/books/RayTracingTheNextWeek.html#boundingvolumehierarchies
    pub fn hit(&self, ray: &Ray, mut tmin: f64, mut tmax: f64) -> bool {
        for i in 0..3 {
            let inv_dir = 1.0 / ray.direction[i];
            let mut t0 = (self.bmin[i] - ray.origin[i]) * inv_dir;
            let mut t1 = (self.bmax[i] - ray.origin[i]) * inv_dir;
            if inv_dir < 0.0 {
                swap(&mut t1, &mut t0);
            }
            tmin = f64::max(t0, tmin);
            tmax = f64::min(t1, tmax);

            if tmax <= tmin {
                return false;
            }
        }
        true
    }

    /// Update self such as it encompass itself and other.
    ///
    /// # Example
    /// ```
    /// use pbrt::geom::aabound::AABoundingBox;
    /// use pbrt::geom::vector3::Vector3f;
    /// let mut a = AABoundingBox::new(&Vector3f::new(-1.0, -1.0, -1.0), &Vector3f::new(1.0, 1.0, 1.0));
    /// let b = AABoundingBox::new(&Vector3f::new(0.0, -1.0, -1.0), &Vector3f::new(2.0, 1.0, 1.0));
    /// a.combine_with(&b);
    /// assert_eq!(a.bmin.x, -1.0);
    /// assert_eq!(a.bmax.x, 2.0);
    /// ```
    pub fn combine_with(&mut self, other: &AABoundingBox) -> &mut Self {
        self.bmin.minimize_by(&other.bmin);
        self.bmax.maximize_by(&other.bmax);
        self
    }

    /// Return the AABoundingBox emcompassing a and b.
    ///
    /// # Example
    /// ```
    /// use pbrt::geom::aabound::AABoundingBox;
    /// use pbrt::geom::vector3::Vector3f;
    /// let a = AABoundingBox::new(&Vector3f::new(-1.0, -1.0, -1.0), &Vector3f::new(1.0, 1.0, 1.0));
    /// let b = AABoundingBox::new(&Vector3f::new(0.0, -1.0, -1.0), &Vector3f::new(2.0, 1.0, 1.0));
    /// let c = AABoundingBox::combine(&a, &b);
    /// assert_eq!(c.bmin.x, -1.0);
    /// assert_eq!(c.bmin.y, -1.0);
    /// assert_eq!(c.bmin.z, -1.0);
    /// assert_eq!(c.bmax.x, 2.0);
    /// assert_eq!(c.bmax.y, 1.0);
    /// assert_eq!(c.bmax.z, 1.0);
    /// ```
    pub fn combine(a: &AABoundingBox, b: &AABoundingBox) -> AABoundingBox {
        let mut res = AABoundingBox::new(&Vector3f::max(), &Vector3f::min());
        res.combine_with(a).combine_with(b);
        res
    }
}

impl Transformable<AABoundingBox> for AABoundingBox {
    fn transform(&self, transform: &Transform) -> Self {
        let min = &self.bmin;
        let max = &self.bmax;
        let vertices = vec![
            Vector3f::new(min.x, min.y, min.z),
            Vector3f::new(min.x, min.y, max.z),
            Vector3f::new(min.x, max.y, min.z),
            Vector3f::new(min.x, max.y, max.z),
            Vector3f::new(max.x, max.y, max.z),
            Vector3f::new(max.x, max.y, min.z),
            Vector3f::new(max.x, min.y, max.z),
            Vector3f::new(max.x, min.y, min.z),
        ];

        let mut transformed_min = Vector3f::max();
        let mut transformed_max = Vector3f::min();

        for vertex in vertices.iter() {
            let transformed_point = transform.transform_point_to_world(vertex);
            transformed_min.minimize_by(&transformed_point);
            transformed_max.maximize_by(&transformed_point);
        }

        AABoundingBox::new(&transformed_min, &transformed_max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collide() {
        let bbox = AABoundingBox::new(&Vector3f::new(-1.0, -1.0, -1.0), &Vector3f::new(1.0, 1.0, 1.0));
        let tests = vec![
            // Rays parallel to axis
            //
            // Ray origin at frame origin
            //
            (Ray::new(&Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(0.0, 0.0, 1.0)), true),
            (Ray::new(&Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(0.0, 1.0, 0.0)), true),
            (Ray::new(&Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(1.0, 0.0, 0.0)), true),
            (Ray::new(&Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(0.0, 0.0, -1.0)), true),
            (Ray::new(&Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(0.0, -1.0, 0.0)), true),
            (Ray::new(&Vector3f::new(0.0, 0.0, 0.0), &Vector3f::new(-1.0, 0.0, 0.0)), true),
            // Ray origin outside the cube, such as ray doesn't intersect
            //
            (Ray::new(&Vector3f::new(1.01, 0.0, 0.0), &Vector3f::new(0.0, 0.0, 1.0)), false),
            (Ray::new(&Vector3f::new(0.0, 0.0, -1.1), &Vector3f::new(0.0, 1.0, 0.0)), false),
            (Ray::new(&Vector3f::new(0.0, 0.0, 1.01), &Vector3f::new(1.0, 0.0, 0.0)), false),
            (Ray::new(&Vector3f::new(-1.01, 0.0, 0.0), &Vector3f::new(0.0, 0.0, -1.0)), false),
            (Ray::new(&Vector3f::new(1.01, 0.0, -1.01), &Vector3f::new(0.0, -1.0, 0.0)), false),
            (Ray::new(&Vector3f::new(1.10, 1.01, 0.0), &Vector3f::new(-1.0, 0.0, 0.0)), false),
            // Ray origin inside the cube, diagonal rays
            //
            (Ray::new(&Vector3f::new(0.5, -0.5, 0.0), &Vector3f::new(1.0, -1.0, 1.0)), true),
            (Ray::new(&Vector3f::new(0.5, -0.5, 0.0), &Vector3f::new(-0.63, -1.0, 2.13)), true),
            (Ray::new(&Vector3f::new(0.5, -0.5, 0.0), &Vector3f::new(12.2, 0.004, -0.0003)), true),
            // Ray origin outside the cube, diagonal rays
            //
            (Ray::new(&Vector3f::new(-2.0, 0.0, 0.0), &Vector3f::new(3.0, 1.0, 0.0)), true),
            (Ray::new(&Vector3f::new(-2.0, 0.0, 0.0), &Vector3f::new(3.0, 1.0, -2.0)), true),
        ];

        for (ray, expected_res) in tests.iter() {
            assert_eq!(bbox.hit(&ray, 0.0, 1000.0), *expected_res);
        }
    }
}
