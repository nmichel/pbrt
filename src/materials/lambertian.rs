use super::{same_hemisphere, Material, ScatterInfo};
use crate::colors;
use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::pdfs::cosine::CosinePdf;
use crate::pdfs::Pdf;
use crate::spectrum::Spectrum;
use crate::textures::*;
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
        let Intersection { ref p, ref n, .. } = intersection;

        let pdf: CosinePdf = CosinePdf {};
        let local_wi: Vector3f = pdf.generate();
        let wi: Vector3f = intersection.local_to_world(&local_wi);

        let shift_avoid_acne = n * 0.001;
        let scattered_ray = Ray::new(&(p + &shift_avoid_acne), &wi);
        let attenuation = self.albedo.shade(intersection) / std::f64::consts::PI;
        let scattering_pdf = pdf.value(&local_wi);

        return Some(ScatterInfo::new(attenuation, scattered_ray, scattering_pdf));
    }

    fn f(&self, world_wo: &Vector3f, world_wi: &Vector3f, interaction: &Interaction) -> Spectrum {
        let Interaction { ref intersection, .. } = interaction;

        let wi = intersection.world_to_local(world_wi);
        let wo = intersection.world_to_local(world_wo);

        if same_hemisphere(&wo, &wi) {
            self.albedo.shade(intersection) / std::f64::consts::PI
        }
        else {
            colors::BLACK
        }
    }
}
