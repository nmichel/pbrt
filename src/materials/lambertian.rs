use super::{Material, ScatterInfo};
use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::pdfs::cosine::CosinePdf;
use crate::pdfs::Pdf;
use crate::spectrum::Spectrum;
use crate::textures::*;
use crate::utils::random_unit_vector;
use core::panic;
use std::sync::Arc;

pub struct Lambertian {
    albedo: Arc<dyn Texture>,
}

impl Lambertian {
    pub fn new(albedo: Arc<dyn Texture>) -> Self {
        Self { albedo: albedo.clone() }
    }
}

impl Material for Lambertian {
    fn scatter(&self, _ray: &Ray, interaction: &Interaction) -> Option<ScatterInfo> {
        let Interaction { ref intersection, .. } = interaction;
        let Intersection { ref p, ref n, ref wo, .. } = intersection;

        let pdf: CosinePdf = CosinePdf {};
        let local_wi: Vector3f = pdf.generate();
        let wi: Vector3f = intersection.local_to_world(&local_wi);
        let mut local_wo = intersection.world_to_local(&wo);
        local_wo.normalize();

        let shift_avoid_acne = n * 0.001;
        let scattered_ray = Ray::new(&(p + &shift_avoid_acne), &wi);
        let attenuation = self.albedo.shade(intersection) / std::f64::consts::PI;
        let scattering_pdf = pdf.value(&local_wi);

        return Some(ScatterInfo::new(attenuation, scattered_ray, scattering_pdf));
    }
}

fn compute_pdf(wo: &Vector3f, wi: &Vector3f) -> f64 {
    if same_hemisphere(wo, wi) {
        abs_cos_theta(wi) / std::f64::consts::PI
    }
    else {
        println!("Lambertian::pdf: wo and wi are not in the same hemisphere");
        0.0
    }
}

fn same_hemisphere(w: &Vector3f, wp: &Vector3f) -> bool {
    w.z * wp.z > 0.0
}

fn abs_cos_theta(w: &Vector3f) -> f64 {
    w.z.abs()
}
