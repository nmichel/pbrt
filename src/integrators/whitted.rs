use crate::geom::intersectable::Intersection;
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::interaction::Interaction;
use crate::materials::material::Material;
use crate::scene::Scene;
use crate::spectrum::Spectrum;

use super::integrator::Integrator;

pub struct WhittedIntegrator {
    // Max recursion depth
    max_depth: usize
}

impl WhittedIntegrator {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }
}

impl Integrator for WhittedIntegrator {
    fn li(&self, ray: &Ray, scene: &Scene, depth: usize) -> Spectrum {
        match scene.intersect(&ray) {
            Some(interaction) => {
                let Interaction { ref intersection, .. } = interaction;
                let Intersection { ref n, ref wo, .. } = intersection;
               
                intersection.le(wo) // Object may be an emitter
                + self.compute_direct_lighting(scene, &interaction) // Add direct contribution
                + self.compute_indirect_lighting(scene, &interaction, depth) // Add indirect contribution
            },
            None => {
                scene.background_radiance(&ray) // In the book, part of the integrator
            }
        }
    }
}

impl WhittedIntegrator {
    fn compute_direct_lighting(&self, scene: &Scene, interaction: &Interaction) -> Spectrum {
        let Interaction { ref intersection, ref material } = interaction;
        let Intersection { ref n, .. } = intersection;
        let black = Spectrum::new(0.0, 0.0, 0.0);

        // Add contribution of each light source
        scene.lights.iter().fold(black, |acc, light| {
            let (ref li, ref wi, ref tester) = light.li(&intersection);
            let f = material.shade(&intersection); // isect.bsdf->f(wo, wi);
            if tester.unoccluded(&scene) {
                acc + (&f * li) * vector3::dot(&wi, n).abs()
            }
            else {
                acc
            }
        })
    }

    fn compute_indirect_lighting(&self, scene: &Scene, interaction: &Interaction, depth: usize) -> Spectrum {
        if depth == 0 {
            return Spectrum::new(0.0, 0.0, 0.0);
        }

        let intersection = &(interaction.intersection);
        let Intersection {ref wo, .. } = intersection;

        let local_wo = intersection.world_to_local(wo);
        let local_wi = vector3::Vector3::new(-local_wo.x, -local_wo.y, local_wo.z);
        let cos_theta = local_wi.z;
        let wi = intersection.local_to_world(&local_wi);

        let o = &intersection.p + &(&(intersection.n) * 0.001);
        let r = Ray::new(&o, &wi);
        let li = self.li(&r, &scene, depth - 1);

        let f = Spectrum::new(0.2, 0.2, 0.2);

        f * &li * cos_theta.abs()
    }
}
