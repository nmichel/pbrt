use std::rc::Rc;
use crate::geom::intersectable::Intersection;
use crate::geom::vector3::{dot, Vector3f};
use crate::spectrum::Spectrum;
use crate::textures::Texture;
use std::f64::consts::PI;

pub struct Material {
    pub texture: Rc<Texture>
}

impl Material {
    pub fn new(texture: Rc<Texture>) -> Self {
        Self { texture }
    }
}

impl Material {
    pub fn shade(&self, intersection: &Intersection, world_wi: &Vector3f) -> Spectrum {
        let diffuse = self.texture.shade(intersection);
        let specular = Spectrum::new(1.0, 1.0, 1.0);
        let spec_coef = Self::cook_torrance(&intersection, &world_wi);
        // println!("spec_coef {:?}", &spec_coef);
        diffuse * Self::lambert() * (1.0 - spec_coef) 
        + specular * spec_coef
        // + specular * Self::phong(&intersection, &world_wi)
        // + specular * Self::blinn_phong(&intersection, &world_wi)

    }

    fn lambert() -> f64 {
        1.0 / 3.14159
    }

    fn phong(intersection: &Intersection, world_wi: &Vector3f) -> f64 {
        let Intersection {ref wo, .. } = intersection;
        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let mut local_wi = intersection.world_to_local(&world_wi);
        local_wi.normalize();
        let r = Vector3f::new(-local_wi.x, -local_wi.y, local_wi.z);
        let spec_coef = num_traits::clamp(dot(&local_wo, &r), 0.0, 1.0);
        spec_coef.powf(20.0)
    }

    fn blinn_phong(intersection: &Intersection, world_wi: &Vector3f) -> f64 {
        let Intersection {ref wo, .. } = intersection;
        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let mut local_wi = intersection.world_to_local(&world_wi);
        local_wi.normalize();
        let mut h = local_wo + local_wi;
        h.normalize();
        let spec_coef = num_traits::clamp(h.z, 0.0, 1.0);
        spec_coef.powf(80.0)
    }

    fn cook_torrance(intersection: &Intersection, world_wi: &Vector3f) -> f64 {
        let Intersection {ref wo, .. } = intersection;
        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let mut local_wi = intersection.world_to_local(&world_wi);
        local_wi.normalize();
        let nl = local_wi.z;
        let nv = local_wo.z;

        let d = Material::d_ggx(&intersection, world_wi);
        // let g = Material::g_cook_torrance(&intersection, world_wi);
        let g = Material::g_smith(&intersection, world_wi);
        let f = Material::f_schlick(&intersection, world_wi);
        //(d * g * f) / (4.0 * nl * nv)
        (d * g * f) / (4.0 * nl * nv)
    }

    fn d_ggx(intersection: &Intersection, world_wi: &Vector3f) -> f64 {
        let Intersection {ref wo, ref n, .. } = intersection;
        let alpha = 0.5;

        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let mut local_wi = intersection.world_to_local(&world_wi);
        local_wi.normalize();
        let mut m = local_wo + local_wi;
        m.normalize();

        let alpha_2 = alpha * alpha; // α²
        let m_z = f64::max(0.0, m.z);
        let m_z_2 = m_z * m_z; // (n.m)² with m in local frame, where n is local z unit vector [0, 0, 1] 
        let den = m_z_2 * (alpha_2 - 1.0) + 1.0; // (n⋅m)²(α² - 1) + 1  
        alpha_2 / (std::f64::consts::PI * den*den) // α² / (π((n⋅m)²(α² - 1) + 1)²)
    }

    fn g_cook_torrance(intersection: &Intersection, world_wi: &Vector3f) -> f64 {
        let Intersection {ref wo, .. } = intersection;

        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let mut local_wi = intersection.world_to_local(&world_wi);
        local_wi.normalize();
        let mut h = local_wo + local_wi;
        h.normalize();

        let vh = dot(&local_wo, &h);
        let nh = h.z;
        let nv = local_wo.z;
        let nl = local_wi.z;
        let t1 = (2.0 * nh * nv) / vh; // (2(n⋅h)(n⋅v))/(v⋅h)
        let t2 = (2.0 * nh * nl) / vh; // (2(n⋅h)(n⋅l))/(v⋅h)
        f64::min(f64::min(1.0, t1), t2)
    }

    fn g_smith(intersection: &Intersection, world_wi: &Vector3f) -> f64 {
        let alpha = 0.5;

        let Intersection {ref wo, .. } = intersection;
        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let mut local_wi = intersection.world_to_local(&world_wi);
        local_wi.normalize();

        Material::g_schlick(&local_wo, alpha) * Material::g_schlick(&local_wi, alpha)
    }

    fn g_schlick(v: &Vector3f, alpha: f64) -> f64 {
        let k = alpha * (2.0 / std::f64::consts::PI).sqrt();
        let nv = f64::max(v.z, 0.0);
        nv / (nv * (1.0 - k) + k)
    }

    fn f_schlick(intersection: &Intersection, world_wi: &Vector3f) -> f64 {
        let Intersection {ref wo, .. } = intersection;

        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let mut local_wi = intersection.world_to_local(&world_wi);
        local_wi.normalize();
        let mut h = local_wo + local_wi;
        h.normalize();

        let ior = 1.8;
        let ior_sub = 1.0 - ior;
        let ior_add = 1.0 + ior;
//        let f0 = (ior_sub * ior_sub) / (ior_add * ior_add);
        let f0 = 0.95;
        let r = f0 + (1.0 - f0) * (1.0 - dot(&local_wo, &h)).powf(5.0);
        r
    }
}
