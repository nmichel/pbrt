use super::ray::Ray;
use super::transform::{Transformable, Transform};
use super::vector3::Vector3f;
use std::mem::swap;

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
        let transformed_point =transform.transform_point_to_world(vertex);
        transformed_min.x = transformed_min.x.min(transformed_point.x);
        transformed_min.y = transformed_min.y.min(transformed_point.y);
        transformed_min.z = transformed_min.z.min(transformed_point.z);

        transformed_max.x = transformed_max.x.max(transformed_point.x);
        transformed_max.y = transformed_max.y.max(transformed_point.y);
        transformed_max.z = transformed_max.z.max(transformed_point.z);
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
