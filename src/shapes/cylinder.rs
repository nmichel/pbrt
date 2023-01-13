use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, Intersection, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use std::f64::consts::PI;

use super::Shape;

pub struct Cylinder {
    pub radius: f64,
    pub height: f64,
    half_height: f64,
}

impl Cylinder {
    pub fn new(radius: f64, height: f64) -> Self {
        Self {
            radius,
            height,
            half_height: height / 2.0,
        }
    }
}

impl Shape for Cylinder {}

impl Intersectable for Cylinder {
    fn intersect(&self, ray: &crate::geom::ray::Ray, near: f64, far: f64) -> IntersectionResult {
        // Borrowed from https://www.pbr-book.org/3ed-2018/Utilities/Mathematical_Routines#Quadratic

        let Ray { ref origin, ref direction } = ray;
        let a: f64 = direction.x * direction.x + direction.z * direction.z;
        let b: f64 = 2.0 * (origin.x * direction.x + origin.z * direction.z);
        let c: f64 = origin.x * origin.x + origin.z * origin.z - self.radius * self.radius;

        let discrim: f64 = b * b - 4.0 * a * c;
        if discrim < 0.0 {
            return IntersectionResult::new();
        }

        let root_discrim: f64 = discrim.sqrt();
        let q: f64 = if b < 0.0 { -0.5 * (b - root_discrim) } else { -0.5 * (b + root_discrim) };
        let t0: f64 = q / a;
        let t1: f64 = c / q;

        let tmin = f64::min(t0, t1);
        let tmax = f64::max(t0, t1);

        if tmax < near || tmin > far {
            return IntersectionResult::new();
        }

        let mut res = IntersectionResult::new();

        if tmin > near {
            let details = self.compute_cynlinder_intersection_details(ray, tmin);
            if details.p.y > -self.half_height && details.p.y < self.half_height {
                res.push(details)
            }
        }

        if tmax > near && tmax < far {
            let details = self.compute_cynlinder_intersection_details(ray, tmax);
            if details.p.y > -self.half_height && details.p.y < self.half_height {
                res.push(details)
            }
        }

        if res.len() < 2 {
            let inv_dir_y = 1.0 / ray.direction.y;

            let t0 = (-self.half_height - ray.origin.y) * inv_dir_y;
            if t0 > near && t0 < far {
                let p = &ray.origin + &(&ray.direction * t0);
                if p.x * p.x + p.z * p.z <= self.radius * self.radius {
                    res.push(Intersection {
                        p,
                        d: t0,
                        n: Vector3f::new(0.0, -1.0, 0.0),
                        wo: &ray.direction * -1.0,
                        u: p.x,
                        v: p.z,
                        dpdu: Vector3f::new(-1.0, 0.0, 0.0),
                        dpdv: Vector3f::new(0.0, 0.0, 1.0),
                    })
                }
            }

            let t1 = (self.half_height - ray.origin.y) * inv_dir_y;
            if t1 > near && t1 < far {
                let p = &ray.origin + &(&ray.direction * t1);
                if p.x * p.x + p.z * p.z <= self.radius * self.radius {
                    res.push(Intersection {
                        p,
                        d: t1,
                        n: Vector3f::new(0.0, 1.0, 0.0),
                        wo: &ray.direction * -1.0,
                        u: p.x,
                        v: p.z,
                        dpdu: Vector3f::new(1.0, 0.0, 0.0),
                        dpdv: Vector3f::new(0.0, 0.0, 1.0),
                    })
                }
            }
        }

        res.sort_by(|a, b| a.d.partial_cmp(&b.d).unwrap());

        res
    }

    fn contain_point(&self, point: &crate::geom::vector3::Vector3f) -> bool {
        point.x * point.x + point.z * point.z < self.radius * self.radius // && point.y > -self.half_height && point.y < self.half_height
    }
}

impl Cylinder {
    fn compute_cynlinder_intersection_details(&self, ray: &Ray, t: f64) -> Intersection {
        let hit = &ray.origin + &(&ray.direction * t);
        let mut normal = Vector3f::new(hit.x, 0.0, hit.z);
        normal.normalize();

        let mut phi = hit.z.atan2(hit.x);
        if phi < 0.0 {
            phi += 2.0 * PI;
        }

        Intersection {
            p: hit,
            d: t,
            n: normal,
            wo: &ray.direction * -1.0,
            u: phi / (2.0 * PI),
            v: hit.y,
            dpdu: Vector3f::new(-hit.z, 0.0, hit.x) * (2.0 * PI),
            dpdv: Vector3f::new(0.0, 1.0, 0.0),
        }
    }
}

impl AABound for Cylinder {
    fn get_bounding_box(&self) -> AABoundingBox {
        let bmin = Vector3f::new(-self.radius, -self.half_height, -self.radius);
        let bmax = Vector3f::new(self.radius, self.half_height, self.radius);
        AABoundingBox::new(&bmin, &bmax)
    }
}
