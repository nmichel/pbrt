use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::spectrum::Spectrum;
use crate::textures::*;
use crate::utils::random_double;
use std::sync::Arc;

pub trait Material : Send + Sync {
    fn scatter(&self, ray: &Ray, interaction: &Interaction) -> Option<(Spectrum, Ray)>;
}

pub struct Lambertian {
    albedo: Arc<Texture>
}

impl Lambertian {
    pub fn new(albedo: Arc<Texture>) -> Self {
        Self { albedo }
    }
}

impl Material for Lambertian {
    fn scatter(&self, _ray: &Ray, interaction: &Interaction) -> Option<(Spectrum, Ray)> {
        let Interaction { ref intersection, .. } = interaction;
        let Intersection { ref p, ref n, .. } = intersection;
        let scatter_dir = n + &random_in_unit_sphere();
        let scattered_ray = Ray::new(p, &scatter_dir);
        let attenuation = self.albedo.shade(intersection);
        return Some((attenuation, scattered_ray));
    }
}

pub struct Metal {
    fuzz: f64,
    albedo: Arc<Texture>
}

impl Metal {
    pub fn new(fuzz: f64, albedo: Arc<Texture>) -> Self {
        Self { fuzz, albedo }
    }
}

impl Material for Metal {
    fn scatter(&self, _ray: &Ray, interaction: &Interaction) -> Option<(Spectrum, Ray)> {
        let Interaction { ref intersection, .. } = interaction;
        let Intersection { ref p, ref n, ref wo, .. } = intersection;

        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let local_reflected = Vector3f::new(-local_wo.x, -local_wo.y, local_wo.z);
        let mut local_target = local_reflected + random_in_unit_sphere() * self.fuzz;
        local_target.normalize();

        if local_target.z > 0.0 { // <=> dot(local_target, n)
            let target = intersection.local_to_world(&local_target);
            let scattered_ray = Ray::new(p, &target);
            let attenuation = self.albedo.shade(intersection);
            Some((attenuation, scattered_ray))
        }
        else {
            None
        }
    }
}


pub struct Dielectric {
    ref_idx: f64,
    albedo: Arc<Texture>
}

impl Dielectric {
    pub fn new(ref_idx: f64, albedo: Arc<Texture>) -> Self {
        Self { ref_idx, albedo }
    }
}

impl Material for Dielectric {
    fn scatter(&self, _ray: &Ray, interaction: &Interaction) -> Option<(Spectrum, Ray)> {
        let Interaction { ref intersection, .. } = interaction;
        let Intersection { ref p, ref n, ref wo, .. } = intersection;

        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();
        let local_reflected = Vector3f::new(-local_wo.x, -local_wo.y, local_wo.z);

        let attenuation = self.albedo.shade(intersection);       
        let local_outward_normal: Vector3f;
        let ni_over_nt: f64;

        let mut cosine: f64;

        if local_wo.z <= 0.0 {
            // Ray's leaving volume
            local_outward_normal = Vector3f::new(0.0, 0.0, -1.0);
            ni_over_nt = self.ref_idx;
            cosine = -local_wo.z;
            cosine = (1.0 - self.ref_idx * self.ref_idx * (1.0 - cosine*cosine)).sqrt();
        }
        else {
            // Ray's entering volume
            local_outward_normal = Vector3f::new(0.0, 0.0, 1.0);
            ni_over_nt = 1.0 / self.ref_idx;
            cosine = local_wo.z;
        }
        match refract(&local_wo, &local_outward_normal, ni_over_nt) {
            Some(local_refracted) => {
                let reflect_prob = schlick(cosine, self.ref_idx);
                let local_scatter_direction = if random_double() < reflect_prob {
                    local_reflected
                }
                else {
                    local_refracted
                };
                let scatter_direction = intersection.local_to_world(&local_scatter_direction);
                let scattered_ray = Ray::new(&(p + &(&scatter_direction * 0.001)), &scatter_direction);
                Some((attenuation, scattered_ray))
            },
            None => {
                // Total reflection
                let target = intersection.local_to_world(&local_reflected);
                let scattered_ray = Ray::new(p, &target);
                Some((attenuation, scattered_ray))
            }
        }
    }
}

fn refract(wi: &Vector3f, n: &Vector3f, ni_over_nt: f64) -> Option<Vector3f> {
    let cos_theta_i = vector3::dot(&wi, n);
    let sin2_theta_i = 1.0 - cos_theta_i * cos_theta_i;
    let sin2_theta_t = ni_over_nt * ni_over_nt * sin2_theta_i;
    let discriminant = 1.0 - sin2_theta_t;
    if discriminant > 0.0 {
        let cos_theta_t = discriminant.sqrt();
        let mut t = wi * -ni_over_nt + n * (ni_over_nt * cos_theta_i - cos_theta_t);
        t.normalize();
        Some(t)
    }
    else {
        None
    }
}

fn schlick(cosine: f64, ref_idx: f64) -> f64 {
    let r = (1.0 - ref_idx) / (1.0 + ref_idx);
    let r2 = r * r;
    return r2 + (1.0 - r2)*(1.0 - cosine).powf(5.0);
}

fn random_in_unit_sphere() -> Vector3f {
    let one = Vector3f::new(1.0, 1.0, 1.0);
    loop {
        let p = Vector3f::new(random_double(),random_double(),random_double()) * 2.0 - one;
        if p.squared_length() < 1.0 {
            return p;
        }
    } 
}
/*
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
*/
