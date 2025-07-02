use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, Intersection, IntersectionResult};
use crate::geom::ray::Ray;
use crate::geom::vector2::Vector2f;
use crate::geom::vector3::{self, Vector3f};
use crate::shapes::Shape;

use super::bvh::{Accumulator, BVHTree};

struct TriangleAccumulator {
    triangles: Vec<usize>,
}

impl TriangleAccumulator {
    pub fn new() -> Self {
        Self { triangles: Vec::new() }
    }
}

impl Accumulator for TriangleAccumulator {
    fn accumulate(&mut self, items: &Vec<usize>) -> () {
        self.triangles.extend(items)
    }
}

pub struct TriangleMesh {
    bvh: BVHTree,
    indices: Vec<usize>,
    vertices: Vec<f64>,
    normals: Option<Vec<f64>>,
    uvs: Option<Vec<f64>>,
}

impl TriangleMesh {
    pub fn new(vertices: Vec<f64>, indices: Vec<usize>, normals: Option<Vec<f64>>, uvs: Option<Vec<f64>>) -> Self {
        let bvh = BVHTree::new(&vertices, &indices);

        Self {
            bvh,
            indices,
            vertices,
            normals,
            uvs,
        }
    }
}

impl Shape for TriangleMesh {}

static DEFAULT_UV0: Vector2f = Vector2f { x: 0.0, y: 0.0 };
static DEFAULT_UV1: Vector2f = Vector2f { x: 1.0, y: 0.0 };
static DEFAULT_UV2: Vector2f = Vector2f { x: 1.0, y: 1.0 };

impl Intersectable for TriangleMesh {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        let mut acc: TriangleAccumulator = TriangleAccumulator::new();
        self.bvh.query(ray, near, far, &mut acc);

        let mut min_t = f64::MAX;
        let mut res: Box<IntersectionResult> = Box::new(IntersectionResult::new());

        for i in (0..acc.triangles.len()) {
            let base = acc.triangles[i];
            // Triangle vertices indices in data arrays
            let i0 = self.indices[base] as usize;
            let i1 = self.indices[base + 1] as usize;
            let i2 = self.indices[base + 2] as usize;

            // Vertices 3D coords
            let ip0 = i0 * 3;
            let ip1 = i1 * 3;
            let ip2 = i2 * 3;

            let coords: &[f64] = &self.vertices;
            let p0 = Vector3f::new(coords[ip0], coords[ip0 + 1], coords[ip0 + 2]);
            let p1 = Vector3f::new(coords[ip1], coords[ip1 + 1], coords[ip1 + 2]);
            let p2 = Vector3f::new(coords[ip2], coords[ip2 + 1], coords[ip2 + 2]);

            let intersection_opt = intersect_ray(ray, &p0, &p1, &p2);
            if intersection_opt.is_none() {
                continue;
            }

            // Compute the intersection point (if any)
            let intersection: TriangleIntersection = intersection_opt.unwrap();

            if intersection.t < near || intersection.t > far {
                continue; // Intersection is outside the ray segment
            }

            if intersection.t >= min_t {
                continue;
            }

            let hit = &ray.origin + &(&ray.direction * intersection.t);

            // Triangle vertices uvs
            let (uv0, uv1, uv2) = if let Some(uvs) = &self.uvs {
                let iuv0 = i0 * 2;
                let iuv1 = i1 * 2;
                let iuv2 = i2 * 2;

                let p0 = Vector2f::new(uvs[iuv0], uvs[iuv0 + 1]);
                let p1 = Vector2f::new(uvs[iuv1], uvs[iuv1 + 1]);
                let p2 = Vector2f::new(uvs[iuv2], uvs[iuv2 + 1]);
                (p0, p1, p2)
            }
            else {
                (DEFAULT_UV0, DEFAULT_UV1, DEFAULT_UV2)
            };

            // Interpolate texture coordinates of intersection point
            let uv = interpolate_texture_coords(&intersection, &uv0, &uv1, &uv2);

            // Compute texture derivatives
            let (dpdu, dpdv) = compute_texture_derivatives(&p0, &p1, &p2, &uv0, &uv1, &uv2);

            let mut result = IntersectionResult::new();
            result.push(Intersection {
                p: hit,
                d: intersection.t,
                n: vector3::cross(&dpdv, &dpdu).normalized(),
                wo: &ray.direction * -1.0,
                u: uv.x,
                v: uv.y,
                dpdu,
                dpdv,
            });

            res = Box::new(result);
            min_t = intersection.t;
        }

        *res
    }

    fn contain_point(&self, _point: &Vector3f) -> bool {
        false
    }
}

impl AABound for TriangleMesh {
    fn get_bounding_box(&self) -> crate::geom::aabound::AABoundingBox {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut min_z = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        let mut max_z = f64::MIN;

        for i in (0..self.vertices.len()).step_by(3) {
            let x = self.vertices[i];
            let y = self.vertices[i + 1];
            let z = self.vertices[i + 2];

            min_x = x.min(min_x);
            min_y = y.min(min_y);
            min_z = z.min(min_z);
            max_x = x.max(max_x);
            max_y = y.max(max_y);
            max_z = z.max(max_z);
        }

        AABoundingBox::new(&Vector3f::new(min_x, min_y, min_z), &Vector3f::new(max_x, max_y, max_z))
    }
}

struct TriangleIntersection {
    t: f64,
    u: f64,
    v: f64,
    w: f64,
}

fn intersect_ray(ray: &Ray, p0: &Vector3f, p1: &Vector3f, p2: &Vector3f) -> Option<TriangleIntersection> {
    // Möller–Trumbore algorithm for ray-triangle intersection
    // https://en.wikipedia.org/wiki/M%C3%B6ller%E2%80%93Trumbore_intersection_algorithm
    //
    // Compute barycentric coordinates u, v (and w as 1 - u - v)
    // and t for the intersection point

    let edge1 = p1 - p0;
    let edge2 = p2 - p0;
    let direction_cross_edge2 = vector3::cross(&ray.direction, &edge2);
    let det = vector3::dot(&edge1, &direction_cross_edge2);

    if det.abs() < 1e-8 {
        return None;
    }

    let inv_det = 1.0 / det;
    let s = &ray.origin - p0;
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

fn interpolate_texture_coords(intersection: &TriangleIntersection, uv0: &Vector2f, uv1: &Vector2f, uv2: &Vector2f) -> Vector2f {
    let u = intersection.w * uv0.x + intersection.u * uv1.x + intersection.v * uv2.x;
    let v = intersection.w * uv0.y + intersection.u * uv1.y + intersection.v * uv2.y;

    Vector2f::new(u, v)
}

fn compute_texture_derivatives(p0: &Vector3f, p1: &Vector3f, p2: &Vector3f, uv0: &Vector2f, uv1: &Vector2f, uv2: &Vector2f) -> (Vector3f, Vector3f) {
    let duv02 = uv0 - uv2;
    let duv12 = uv1 - uv2;

    let dp02 = p0 - p2;
    let dp12 = p1 - p2;

    let det = duv12.x * duv02.y - duv12.y * duv02.x;

    if det.abs() < 1e-8 {
        // Degenerate case : colinear uv coordinates
        // We can compute a normal and two orthogonal vectors

        let normal = vector3::cross(&dp12, &dp02);
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
        let dpdu = &(&dp12 * duv02.y - &dp02 * duv12.y) * inv_det;
        let dpdv = &(&dp02 * duv12.x - &dp12 * duv02.x) * inv_det;
        (dpdu, dpdv)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_triangle_mesh_creation() {
        let vertices = vec![-1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0];
        let indices = vec![0, 1, 2];
        let normals = Some(vec![0.0, 0.0, 1.0]);
        let uvs = Some(vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0]);

        let mesh = TriangleMesh::new(vertices, indices, normals, uvs);
    }
}
