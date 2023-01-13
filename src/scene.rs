use super::bvh::{Accumulator, BVHNode};
use super::geom::aabound::{AABound, AABoundingBox};
use super::geom::intersectable::{Intersectable, Intersection, IntersectionResult};
use super::geom::ray::Ray;
use super::geom::vector3::Vector3f;
use super::interaction::Interaction;
use super::light::Light;
use super::primitives::Primitive;
use std::sync::Arc;

struct Wrapper<T>(Arc<T>)
where
    T: Intersectable + AABound;

impl<T> Intersectable for Wrapper<T>
where
    T: Intersectable + AABound,
{
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        self.0.intersect(ray, near, far)
    }

    fn contain_point(&self, point: &Vector3f) -> bool {
        self.0.contain_point(point)
    }
}

impl<T> Clone for Wrapper<T>
where
    T: Intersectable + AABound,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> AABound for Wrapper<T>
where
    T: Intersectable + AABound,
{
    fn get_bounding_box(&self) -> AABoundingBox {
        self.0.get_bounding_box()
    }
}

pub struct Scene {
    primitives: Vec<Wrapper<Primitive>>,
    lights: Vec<Arc<dyn Light>>,
    bvh: Option<BVHNode<Wrapper<Primitive>>>,
}

impl Scene {
    pub fn new() -> Scene {
        Scene {
            primitives: Vec::new(),
            lights: Vec::new(),
            bvh: None,
        }
    }

    pub fn add_object(&mut self, object: Arc<Primitive>) -> &mut Self {
        self.primitives.push(Wrapper(Arc::clone(&object)));
        self
    }

    pub fn add_light(&mut self, light: Arc<dyn Light>) -> &mut Self {
        // Arc::get_mut(&mut self.lights).unwrap().push(light);
        self.lights.push(light);
        self
    }

    pub fn commit(&mut self) -> &mut Self {
        let bvh = Self::build_bvh(&mut self.primitives);
        self.primitives = vec![];
        self.bvh = bvh;
        self
    }

    pub fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Interaction> {
        let mut accumulator = PrimitiveAccumulator { acc: Vec::new() };

        match &self.bvh {
            None => None,
            Some(bvh_node) => {
                // Ask the acceleration structure for potentially hit items
                bvh_node.query(ray, near, far, &mut accumulator);

                // Iterate through candidates to find the nearest interaction
                accumulator.acc.iter().fold(Option::<Interaction>::None, |acc, primitive| {
                    let intersections = primitive.intersect(ray, near, far);

                    let slice: &[Intersection] = intersections.as_slice();
                    match slice {
                        [intersection, ..] => {
                            match acc {
                                None => {
                                    let material = primitive.0.material.clone();
                                    Some(Interaction {
                                        intersection: *intersection,
                                        material,
                                    })
                                }
                                Some(ref prev_interaction) => {
                                    if intersection.d < prev_interaction.intersection.d {
                                        let material = primitive.0.material.clone();
                                        Some(Interaction {
                                            intersection: *intersection,
                                            material,
                                        })
                                    }
                                    else {
                                        acc
                                    }
                                }
                            }
                        }
                        _ => acc,
                    }
                })
            }
        }
    }

    fn build_bvh(prim: &mut Vec<Wrapper<Primitive>>) -> Option<BVHNode<Wrapper<Primitive>>> {
        Some(BVHNode::new(prim))
    }
}

struct PrimitiveAccumulator {
    pub acc: Vec<Wrapper<Primitive>>,
}

impl Accumulator<Wrapper<Primitive>> for PrimitiveAccumulator {
    fn accumulate(&mut self, items: &mut Vec<Wrapper<Primitive>>) -> () {
        self.acc.append(items);
        ()

        // let slice: &[Intersection] = intersections.as_slice();
        // match slice {
        //     [intersection, ..] => {
        //         match &mut self.acc {
        //             None => {
        //                 let inter = *intersection;
        //                 let material = item.0.material.clone();
        //                 self.acc = Some(Interaction {
        //                     intersection: inter,
        //                     material,
        //                 })
        //             }
        //             Some(ref prev_interaction) => {
        //                 if intersection.d < prev_interaction.intersection.d {
        //                     let material = item.0.material.clone();
        //                     self.acc = Some(Interaction {
        //                         intersection: *intersection,
        //                         material,
        //                     })
        //                 }
        //                 else {
        //                     ()
        //                 }
        //             }
        //         }
        //     }
        //     _ => (),
        // }
        // ()
    }
}
