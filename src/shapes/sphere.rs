use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, Intersection, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::geom::vector3::Vector3f;
use num_traits::clamp;
use std::f64::consts::PI;

use super::Shape;

pub struct Sphere {
    r: f64,
}

impl Sphere {
    pub fn new(r: f64) -> Sphere {
        Sphere { r }
    }

    pub fn radius(&self) -> f64 {
        self.r
    }
}

impl Shape for Sphere {}

impl Intersectable for Sphere {
    /// See https://www.pbr-book.org/3ed-2018/Shapes/Spheres
    /// see https://en.wikipedia.org/wiki/Spherical_coordinate_system
    /// See https://en.wikipedia.org/wiki/Chain_rule
    ///
    /// θ
    /// φ
    /// π
    /// δ
    ///
    /// 3D space to sperical coordinates
    /// ---
    ///
    /// φ = atan2(y, x)
    /// θ = acos(z/r)
    ///
    /// x = r sin(θ) cos(φ)   [1]
    /// y = r sin(θ) sin(φ)   [2]
    /// z = r cos(θ)
    ///
    /// Angles to u/v mapping
    /// ---
    ///
    /// φ in [0, 2π]
    /// θ in [0, π]
    /// u, v in [0, 1]
    ///
    /// φ(u) = 2π u
    /// θ(v) = π v
    ///
    /// Projection of intersection point / basic trigonometry
    /// ---
    ///
    /// cos(θ) = z/r   [3]
    /// cos(φ) = x/r   [4]
    /// sin(φ) = y/r   [5]
    ///
    /// Derivatives of (u, v) position (using Chain Rule)
    /// ---
    ///
    /// δx/δθ = δ(r sin(θ) cos(φ))/δθ
    ///       = r cos(θ) cos(φ)
    ///       = r (z/r) cos(φ)   [3]
    ///       = z cos(φ)
    ///
    /// δy/δθ = r cos(θ) sin(φ)
    ///       = r (z/r) sin(φ)   [3]
    ///       = z sin(φ)
    ///
    /// δz/δθ = -r sin(θ)
    ///
    /// δx/δφ = -r sin(θ) sin(φ)
    ///       = -y     [2]
    ///
    /// δy/δφ = r sin(θ) cos(φ)
    ///       = x       [1]
    ///
    /// δz/δφ = 0
    ///
    /// θ(v) = π v
    /// δθ/δv = π
    /// δx/δv = δx/δθ δθ/δv                           [Chain Rule]
    ///       = (r cos(θ) cos(φ)) δθ/δv
    ///       = π z cos(φ)
    /// δy/δv = δy/δθ δθ/δv
    ///       = π z sin(φ)
    /// δz/δv = -r π sin(θ)
    ///
    /// φ(u) = 2π u
    /// δφ/δu = 2π
    /// δx/δu = δx/δφ δφ/δu                           [Chain Rule]
    ///       = (-r sin(θ) sin(φ)) δφ/δu
    ///       = -2π y
    /// δy/δu = δy/δφ δφ/δu
    ///       = 2π x
    /// δz/δu = 0
    ///
    /// δp/δu = 2π(-y, x, 0)
    /// δp/δv = π(z cos(φ), z sin(φ), -r sin(θ))
    ///       = π(zx/r, zy/r, -r sin(θ))   [4][5]
    ///
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        // Compute intersection point (geometric solution):
        // https://www.scratchapixel.com/lessons/3d-basic-rendering/minimal-ray-tracer-rendering-simple-shapes/ray-sphere-intersection

        let l = &ray.origin * -1.0;
        let tca = vector3::dot(&l, &ray.direction);
        if tca < 0.0 {
            return IntersectionResult::new();
        }

        let r2 = self.r * self.r;
        let d2 = vector3::dot(&l, &l) - tca * tca;
        if d2 > r2 {
            return IntersectionResult::new();
        }

        let thc = (r2 - d2).sqrt();
        let t0: f64 = tca - thc;
        let t1: f64 = tca + thc;
        let tmin = f64::min(t0, t1);
        let tmax = f64::max(t0, t1);
        if tmax < near || tmin > far {
            return IntersectionResult::new();
        }

        let mut res = IntersectionResult::new();

        if tmin > near {
            res.push(self.compute_intersection_details(ray, tmin))
        }

        if tmax > near && tmax < far {
            res.push(self.compute_intersection_details(ray, tmax))
        }

        res
    }

    fn contain_point(&self, point: &Vector3f) -> bool {
        vector3::dot(&point, &point) < self.r * self.r
    }
}

impl Sphere {
    fn compute_intersection_details(&self, ray: &Ray, t: f64) -> Intersection {
        let hit = &ray.origin + &(&ray.direction * t);
        let mut norm = hit;
        norm.normalize();

        // Compute UV coords (<=> polar coords)

        let mut phi = hit.y.atan2(hit.x);
        if phi < 0.0 {
            phi += 2.0 * PI;
        }
        let u = phi / (2.0 * PI);

        let theta = clamp(hit.z / self.r, -1.0, 1.0).acos();
        let v = theta / PI;

        // Compute UV derivatives

        let z_radius = (hit.x * hit.x + hit.y * hit.y).sqrt();
        let inv_z_r = 1.0 / z_radius;
        let cos_phi = hit.x * inv_z_r;
        let sin_phi = hit.y * inv_z_r;
        let dpdu = vector3::Vector3::new(-hit.y, hit.x, 0.0) * (2.0 * PI);
        let dpdv = vector3::Vector3::new(hit.z * cos_phi, hit.z * sin_phi, -self.r * theta.sin()) * PI;

        Intersection {
            p: hit,
            d: t,
            n: norm,
            wo: &ray.direction * -1.0,
            u,
            v,
            dpdu,
            dpdv,
        }
    }
}

impl AABound for Sphere {
    fn get_bounding_box(&self) -> AABoundingBox {
        let bmin = Vector3f::new(-self.r, -self.r, -self.r);
        let bmax = Vector3f::new(self.r, self.r, self.r);
        AABoundingBox::new(&bmin, &bmax)
    }
}
