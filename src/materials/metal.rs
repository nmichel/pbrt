use super::{Material, ScatterInfo};
use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::shading_frame::ShadingFrame;
use crate::geom::vector3::Vector3f;
use crate::interaction::Interaction;
use crate::pdfs::sphere::SpherePdf;
use crate::pdfs::Pdf;
use crate::samplers::Sampler;
use crate::textures::*;
use std::sync::Arc;

pub struct Metal {
    fuzz: f64,
    albedo: Arc<dyn Texture>,
}

impl Metal {
    pub fn new(fuzz: f64, albedo: Arc<dyn Texture>) -> Self {
        Self { fuzz, albedo }
    }
}

impl Material for Metal {
    fn scatter(&self, _ray: &Ray, interaction: &Interaction, sampler: &mut dyn Sampler) -> Option<ScatterInfo> {
        // (1) wo is the opposite of incoming ray (i.e. wo "goes away" from the intersection point),
        // so, wi = -wo.
        //
        // local_wo is expressed in a space where the up vector is 'z' and is also the normal vector to
        // the surface at the intersection point.
        //
        // So, computing the reflection of the wi vector (wi - 2*dot(wi, n)*n) where n is [0, 0, 1]
        // leads to [wix, wiy, -wiz]
        // with wi = -wo the end result is [-wox, -woy, woz]

        let Interaction { ref intersection, .. } = interaction;
        let Intersection { ref p, ref n, ref wo, .. } = intersection;

        let frame = ShadingFrame::from(intersection);
        let mut local_wo = frame.world_to_local(&wo);
        local_wo.normalize();

        let local_reflected = Vector3f::new(-local_wo.x, -local_wo.y, local_wo.z); // (1)

        // `fuzz` blurs the mirror direction by displacing it before renormalising. The offset is
        // drawn on the *surface* of the sphere of radius `fuzz`, so its magnitude is exactly
        // `fuzz`, and `SpherePdf` — which already derives the uniform sphere map — is where that
        // draw comes from.
        //
        // **A departure worth naming.** This blur is not a physical model of a rough conductor: it
        // is the phenomenological `fuzz` of "Ray Tracing in One Weekend", and it has no microfacet
        // distribution, no shadowing-masking term and no Fresnel behind it. Sampling the sphere
        // rather than the ball it bounds is therefore a change *within* an approximation, not a
        // change to a physical quantity — but it is not invisible: a ball-uniform offset has mean
        // magnitude ¾·fuzz, so the perturbation here is about a third larger. At large `fuzz` more
        // targets fall below the horizon and are absorbed by the `local_target.z > 0.0` test
        // below, so a fuzzy metal is slightly darker. Replacing the blur with a microfacet BRDF is
        // the fix that would make the question moot.
        let fuzz_offset = SpherePdf {}.generate(&sampler.get_2d()) * self.fuzz;
        let mut local_target = local_reflected + fuzz_offset;
        local_target.normalize();

        // Above the horizon: in the shading frame the z component *is* cos θ, so this is the test
        // `dot(local_target, n) > 0` without the dot product. A blurred direction that fell below
        // the surface is absorbed rather than reflected into the geometry.
        if local_target.z > 0.0 {
            let target = frame.local_to_world(&local_target);
            let shift_avoid_acne = n * 0.001;
            let scattered_ray = Ray::new(&(p + &shift_avoid_acne), &target);

            // see https://www.pbr-book.org/3ed-2018/Reflection_Models/Specular_Reflection_and_Transmission#SpecularReflection
            // Handling of extra cosine because of delta distribution
            let abs_cos_theta = local_target.z.abs();
            let attenuation = self.albedo.shade(intersection) / abs_cos_theta;

            Some(ScatterInfo::new(attenuation, scattered_ray, 1.0))
        }
        else {
            None
        }
    }

    fn is_specular(&self) -> bool {
        true
    }
}
