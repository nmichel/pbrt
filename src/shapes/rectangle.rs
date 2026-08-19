use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, Intersection, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::geom::vector3::Vector3f;

use super::Shape;

pub struct Rectangle {
    half_width: f64,
    half_height: f64,
}

impl Rectangle {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            half_width: width / 2.0,
            half_height: height / 2.0,
        }
    }
}

impl Shape for Rectangle {}

impl Intersectable for Rectangle {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        let mut res = IntersectionResult::new();

        if ray.direction.y == 0.0 {
            res
        }
        else {
            let d = (ray.origin.y * -1.0) / ray.direction.y;
            if d < near || d > far {
                return res;
            }

            let mut p = ray.origin + ray.direction * d;
            p.y = 0.0;

            if p.x.abs() > self.half_width || p.z.abs() > self.half_height {
                return res;
            }

            res.push(Intersection {
                p,
                d,
                n: vector3::Vector3f::new(0.0, 1.0, 0.0),
                wo: &ray.direction * -1.0,
                u: p.x,
                v: p.z,
                dpdu: vector3::Vector3::new(1.0, 0.0, 0.0),
                dpdv: vector3::Vector3::new(0.0, 0.0, 1.0),
            });

            res
        }
    }

    /// A rectangle is an open surface and encloses nothing, so it contains no point.
    ///
    /// Answering `y <= 0` within the footprint would describe a solid reaching down for ever, and
    /// `get_bounding_box` below — flat at y = 0, correctly, since that is where the surface is —
    /// would then fail to contain it. Making the bound match instead would render every rectangle
    /// unbounded, and the floors, walls and light panels of every scene would leave the accelerator
    /// for a volume they do not have.
    fn contain_point(&self, _point: &Vector3f) -> bool {
        false
    }
}

impl AABound for Rectangle {
    /// The rectangle lies in the y = 0 plane, so its bound is flat along y.
    ///
    /// Flat and not padded, which rests on `AABoundingBox::hit` counting a tangential hit: padding
    /// the bound to, say, y ∈ [-1, 1] would be a two-unit-thick box around a surface with no
    /// thickness, loose enough to drag the rectangle into every node it does not belong to, and it
    /// would report an area it does not have to every split cost that reads it.
    fn get_bounding_box(&self) -> AABoundingBox {
        let bmin = Vector3f::new(-self.half_width, 0.0, -self.half_height);
        let bmax = Vector3f::new(self.half_width, 0.0, self.half_height);
        AABoundingBox::new(&bmin, &bmax)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An open surface encloses nothing, so no point is inside it — and its flat bound is then a
    /// truthful description rather than one that omits a volume.
    #[test]
    fn test_an_open_surface_encloses_nothing() {
        let rectangle = Rectangle::new(2.0, 3.0);

        assert!(!rectangle.contain_point(&Vector3f::new(0.0, -1.0, 0.0)));
        assert!(!rectangle.contain_point(&Vector3f::new(0.0, 0.0, 0.0)));

        let bbox = rectangle.get_bounding_box();
        assert_eq!(bbox.bmin.y, 0.0);
        assert_eq!(bbox.bmax.y, 0.0);
        assert!(bbox.is_bounded());
    }
}
