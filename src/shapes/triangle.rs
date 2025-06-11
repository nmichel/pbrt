use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, Intersection, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::geom::vector3::Vector3f;

use super::Shape;

pub struct Triangle {
    p0: Vector3f,
    p1: Vector3f,
    p2: Vector3f,
}

impl Triangle {
    pub fn new(p0: Vector3f, p1: Vector3f, p2: Vector3f) -> Triangle {
        Triangle { p0, p1, p2 }
    }
}

impl Shape for Triangle {}

impl Intersectable for Triangle {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        // Moller–Trumbore intersection algorithm
        let edge1 = self.p1 - self.p0;
        let edge2 = self.p2 - self.p0;
        let direction_cross_edge2 = vector3::cross(&ray.direction, &edge2);
        let det = vector3::dot(&edge1, &direction_cross_edge2);

        if det.abs() < 1e-8 {
            return IntersectionResult::new();
        }

        let inv_det = 1.0 / det;
        let s = ray.origin - self.p0;
        let u = inv_det * vector3::dot(&s, &direction_cross_edge2);

        if u < 0.0 || u > 1.0 {
            return IntersectionResult::new();
        }

        let s_cross_edge1 = vector3::cross(&s, &edge1);
        let v = inv_det * vector3::dot(&ray.direction, &s_cross_edge1);

        if v < 0.0 || u + v > 1.0 {
            return IntersectionResult::new();
        }

        let t = inv_det * vector3::dot(&edge2, &s_cross_edge1);
        if t < near || t > far {
            return IntersectionResult::new();
        }

        let hit = &ray.origin + &(&ray.direction * t);
        let normal = vector3::cross(&edge1, &edge2).normalized();
        let mut res = IntersectionResult::new();

        let u0: f64 = 0.0;
        let v0: f64 = 0.0;
        let u1: f64 = 1.0;
        let v1: f64 = 0.0;
        let u2: f64 = 1.0;
        let v2: f64 = 1.0;

        let du02 = u0 - u2;
        let dv02 = v0 - v2;
        let du12 = u1 - u2;
        let dv12 = v1 - v2;

        let dp02 = self.p0 - self.p2;
        let dp12 = self.p1 - self.p2;

        let det = du12 * dv02 - dv12 * du02;

        if det.abs() < 1e-8 {
            // Degenerate triangle, cannot compute derivatives
            return res;
        }

        let inv_det = 1.0 / det;
        let dpdu = &(&dp12 * dv02 - &dp02 * dv12) * inv_det;
        let dpdv = &(&dp02 * du12 - &dp12 * du02) * inv_det;

        /*
            let edge12 = p1 - p2;  // ∂M/∂β
            let edge02 = p0 - p2;  // ∂M/∂α

            let duv12 = uv1 - uv2; // (Δu₂, Δv₂)
            let duv02 = uv0 - uv2; // (Δu₁, Δv₁)

            let det = duv12.u * duv02.v - duv12.v * duv02.u;

            if det != 0.0 {
                let inv_det = 1.0 / det;

                let dp_du = inv_det * (duv02.v * edge12 - duv12.v * edge02);
                let dp_dv = inv_det * (duv12.u * edge02 - duv02.u * edge12);
            }
        */

        /*
        Float determinant = DifferenceOfProducts(du02, dv12, dv02, du12);
        cd = dv02 * du12;
        det = du02 * dv12 - dv02 * du12;
        */

        res.push(Intersection {
            p: hit,
            d: t,
            n: normal,
            wo: &ray.direction * -1.0,
            // nonsense : TODO: compute correct UV values
            u,
            v,
            dpdu,
            dpdv,
        });

        res
    }

    fn contain_point(&self, _point: &Vector3f) -> bool {
        false
    }
}

impl AABound for Triangle {
    fn get_bounding_box(&self) -> AABoundingBox {
        let min_x = self.p0.x.min(self.p1.x).min(self.p2.x);
        let min_y = self.p0.y.min(self.p1.y).min(self.p2.y);
        let min_z = self.p0.z.min(self.p1.z).min(self.p2.z);
        let max_x = self.p0.x.max(self.p1.x).max(self.p2.x);
        let max_y = self.p0.y.max(self.p1.y).max(self.p2.y);
        let max_z = self.p0.z.max(self.p1.z).max(self.p2.z);

        AABoundingBox::new(&Vector3f::new(min_x, min_y, min_z), &Vector3f::new(max_x, max_y, max_z))
    }
}
