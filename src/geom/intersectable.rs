use super::ray::Ray;
use super::vector3;
use super::vector3::Vector3f;
use crate::spectrum::Spectrum;
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub struct Intersection {
    /// Intersection point
    pub p: Vector3f,

    /// Distance to the `Ray` origin
    pub d: f64,

    /// Normal vector at intersection point
    pub n: Vector3f,

    /// Inverse direction of the `Ray`
    pub wo: Vector3f,

    /// U/V coordinate of intersection point on the surface
    pub u: f64,
    pub v: f64,

    /// Position derivatives at intersection point
    pub dpdu: Vector3f,
    pub dpdv: Vector3f,
}

pub type IntersectionResult = Vec<Intersection>;

pub trait Intersectable: Send + Sync {
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult;

    /// Whether `ray` meets this geometry at all within `[near, far]`.
    ///
    /// A different question from `intersect`, and one that deserves different work: *any* hit
    /// answers it, so there is no nearest to look for, no distance to rank, and none of the
    /// shading frame — normal, texture coordinates, ∂p/∂u, ∂p/∂v — that a surface whose only role
    /// is to be in the way will never be asked for. Shadow rays ask this and nothing else.
    ///
    /// The default implementation answers it by way of `intersect`, which is **correct for every
    /// implementation and wasteful for most**: it computes all of the above and then looks only at
    /// whether the list came back empty. It is a default so that adding this method breaks
    /// nothing, not because it is a good way to answer. Override it wherever the waste is worth
    /// removing — which means wherever `intersect` does real work, and not in a shape whose
    /// intersection is a handful of arithmetic.
    fn intersect_p(&self, ray: &Ray, near: f64, far: f64) -> bool {
        !self.intersect(ray, near, far).is_empty()
    }

    fn contain_point(&self, point: &Vector3f) -> bool;
}

impl Intersection {
    pub fn le(&self, _wo: &Vector3f) -> Spectrum {
        // match self.primitive.getAreaLight() {
        //     Some(light) => light.l(self, wo),
        //     None => Spectrum::new(0.0, 0.0, 0.0)
        // }
        //
        // Pour l'instant, retourne "rien"
        Spectrum::new(0.0, 0.0, 0.0)
    }

    pub fn world_to_local(&self, v: &Vector3f) -> Vector3f {
        //                  |ss.x ts.x ns.x|
        // local_to_world = |ss.y ts.y ns.y|
        //                  |ss.z ts.z ns.z|
        //
        // wp = local_to_world * lp
        //
        // world_to_local = inv(local_to_world)
        // Rotation matrix : inv(local_to_world) == transpose(local_to_world)
        //
        //                  |ss.x ss.y ss.z|
        // world_to_local = |ts.x ts.y ts.z|
        //                  |ns.z ns.y ns.z|
        //
        //
        // lp = world_to_local * wp

        let ns = self.n;
        let mut ss = self.dpdu;
        ss.normalize();
        let ts = vector3::cross(&ns, &ss);

        vector3::Vector3::new(vector3::dot(&ss, &v), vector3::dot(&ts, &v), vector3::dot(&ns, &v))
    }

    pub fn local_to_world(&self, v: &Vector3f) -> Vector3f {
        //                  |ss.x ts.x ns.x|
        // local_to_world = |ss.y ts.y ns.y|
        //                  |ss.z ts.z ns.z|
        //
        // wp = local_to_world * lp
        //
        // world_to_local = inv(local_to_world)
        // Rotation matrix : inv(local_to_world) == transpose(local_to_world)
        //
        //                  |ss.x ss.y ss.z|
        // world_to_local = |ts.x ts.y ts.z|
        //                  |ns.z ns.y ns.z|
        //
        //
        // lp = world_to_local * wp

        let ns = self.n;
        let mut ss = self.dpdu;
        ss.normalize();
        let ts = vector3::cross(&ns, &ss);

        vector3::Vector3::new(
            ss.x * v.x + ts.x * v.y + ns.x * v.z,
            ss.y * v.x + ts.y * v.y + ns.y * v.z,
            ss.z * v.x + ts.z * v.y + ns.z * v.z,
        )
    }
}

impl fmt::Display for Intersection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[position: {}, distance: {}, normal: {}]", self.p, self.d, self.n)
    }
}
