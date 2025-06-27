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
        // https://en.wikipedia.org/wiki/M%C3%B6ller%E2%80%93Trumbore_intersection_algorithm

        if let Some(TriangleIntersection { t, u, v, w }) = self.intersect_ray(ray) {
            if t < near || t > far {
                return IntersectionResult::new();
            }

            let hit = &ray.origin + &(&ray.direction * t);

            let edge1 = self.p1 - self.p0;
            let edge2 = self.p2 - self.p0;
            let normal = vector3::cross(&edge1, &edge2).normalized();
            let mut res = IntersectionResult::new();

            let u0: f64 = 0.0;
            let v0: f64 = 0.0;
            let u1: f64 = 1.0;
            let v1: f64 = 0.0;
            let u2: f64 = 1.0;
            let v2: f64 = 1.0;

            let u = w * u0 + u * u1 + v * u2;
            let v = w * v0 + u * v1 + v * v2;

            let du02 = u0 - u2;
            let dv02 = v0 - v2;
            let du12 = u1 - u2;
            let dv12 = v1 - v2;

            let dp02 = self.p0 - self.p2;
            let dp12 = self.p1 - self.p2;

            let det = du12 * dv02 - dv12 * du02;

            let (dpdu, dpdv) = if det.abs() < 1e-8 {
                // Degenerate case : colinear uv coordinates
                // We can compute a normal and two orthogonal vectors

                let normal = vector3::cross(&edge1, &edge2);
                let tangent = if normal.x.abs() < 0.9 {
                    vector3::cross(&Vector3f { x: 1.0, y: 0.0, z: 0.0 }, &normal)
                }
                else {
                    vector3::cross(&Vector3f { x: 0.0, y: 1.0, z: 0.0 }, &normal)
                };
                let bitangent = vector3::cross(&normal, &tangent);
                (tangent, bitangent)
            }
            else {
                // Regular case : we can compute the derivatives
                // using the determinant of the Jacobian matrix

                let inv_det = 1.0 / det;
                let dpdu = &(&dp12 * dv02 - &dp02 * dv12) * inv_det;
                let dpdv = &(&dp02 * du12 - &dp12 * du02) * inv_det;

                (dpdu, dpdv)
            };

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
        else {
            return IntersectionResult::new();
        }
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

struct TriangleIntersection {
    t: f64,
    u: f64,
    v: f64,
    w: f64,
}

impl Triangle {
    fn intersect_ray(&self, ray: &Ray) -> Option<TriangleIntersection> {
        // Compute barycentric coordinates u and v
        // and t for the intersection point

        let edge1 = self.p1 - self.p0;
        let edge2 = self.p2 - self.p0;
        let direction_cross_edge2 = vector3::cross(&ray.direction, &edge2);
        let det = vector3::dot(&edge1, &direction_cross_edge2);

        if det.abs() < 1e-8 {
            return None;
        }

        let inv_det = 1.0 / det;
        let s = ray.origin - self.p0;
        let u = inv_det * vector3::dot(&s, &direction_cross_edge2);

        if u < 0.0 || u > 1.0 {
            return None;
        }

        let s_cross_edge1 = vector3::cross(&s, &edge1);
        let v = inv_det * vector3::dot(&ray.direction, &s_cross_edge1);

        if v < 0.0 || u + v > 1.0 {
            return None;
        }

        let t = inv_det * vector3::dot(&edge2, &s_cross_edge1);

        Some(TriangleIntersection { t, u, v, w: 1.0 - u - v })
    }
}
