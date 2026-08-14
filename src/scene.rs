use crate::lights::LightType;
use crate::objects::Object;

use super::bvh::{Accumulator, BVHNode, TraversalStats};
use super::geom::aabound::{AABound, AABoundingBox};
use super::geom::intersectable::{Intersectable, IntersectionResult};
use super::geom::ray::Ray;
use super::geom::vector3::Vector3f;
use super::interaction::Interaction;
use super::lights::Light;
use std::sync::Arc;

struct Wrapper<T>(Arc<T>)
where
    T: Object + ?Sized;

impl<T> Intersectable for Wrapper<T>
where
    T: Object + ?Sized,
{
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> IntersectionResult {
        Intersectable::intersect(self.0.as_ref(), ray, near, far)
    }

    /// Forwarded like `intersect`: a wrapper that let the default implementation through would
    /// send every occlusion query back down the nearest-hit path it exists to avoid.
    fn intersect_p(&self, ray: &Ray, near: f64, far: f64) -> bool {
        Intersectable::intersect_p(self.0.as_ref(), ray, near, far)
    }

    fn contain_point(&self, point: &Vector3f) -> bool {
        self.0.contain_point(point)
    }
}

impl<T> Clone for Wrapper<T>
where
    T: Object + ?Sized,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> AABound for Wrapper<T>
where
    T: Object + ?Sized,
{
    fn get_bounding_box(&self) -> AABoundingBox {
        self.0.get_bounding_box()
    }
}

impl<T> Object for Wrapper<T>
where
    T: Object + ?Sized,
{
    fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Interaction> {
        Object::intersect(self.0.as_ref(), ray, near, far)
    }
}

pub struct Scene {
    primitives: Vec<Wrapper<dyn Object>>,

    /// Primitives with no finite bounding box — a `Plane` and anything containing one.
    ///
    /// They are kept out of the accelerator and tested for every ray instead. A spatial structure
    /// works by excluding what a ray cannot reach, and a primitive that fills space can never be
    /// excluded: its box overlaps every node, so it would be visited by every ray anyway, while
    /// its infinite area poisoned every split cost it entered. Being outside costs one test per
    /// ray and buys back a tree that means something. pbrt does the same.
    unbounded: Vec<Wrapper<dyn Object>>,

    lights: Vec<Arc<dyn Light>>,
    bvh: Option<BVHNode<Wrapper<dyn Object>>>,
}

impl Scene {
    pub fn new() -> Scene {
        Scene {
            primitives: Vec::new(),
            unbounded: Vec::new(),
            lights: Vec::new(),
            bvh: None,
        }
    }

    pub fn add_object(&mut self, object: Arc<dyn Object>) -> &mut Self {
        self.primitives.push(Wrapper(Arc::clone(&object)));
        self
    }

    pub fn add_light(&mut self, light: Arc<dyn Light>) -> &mut Self {
        self.lights.push(light);
        self
    }

    /// Sorts the primitives added so far into those the accelerator can hold and those it cannot,
    /// then builds the tree over the first group.
    pub fn commit(&mut self) -> &mut Self {
        let mut bounded: Vec<Wrapper<dyn Object>> = Vec::new();

        for primitive in self.primitives.drain(..) {
            if primitive.get_bounding_box().is_bounded() {
                bounded.push(primitive);
            }
            else {
                self.unbounded.push(primitive);
            }
        }

        self.bvh = Self::build_bvh(&mut bounded);
        self
    }

    /// Nearest interaction of `ray` with the scene, within `[near, far]`.
    pub fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Interaction> {
        self.find_nearest(ray, near, far, &mut TraversalStats::default())
    }

    /// Same as [`Scene::intersect`], but adds the work done to `stats`.
    ///
    /// Both entry points run the very same `find_nearest`, so the measured search is the one the
    /// renderer performs.
    pub fn intersect_instrumented(&self, ray: &Ray, near: f64, far: f64, stats: &mut TraversalStats) -> Option<Interaction> {
        self.find_nearest(ray, near, far, stats)
    }

    /// Whether anything at all stands along `ray` within `[near, far]`.
    ///
    /// Not `intersect(..).is_some()`. The question is different, and so is the work it deserves:
    /// *any* hit settles it, so the search stops at the first one instead of ranking every
    /// candidate by distance, and no `Interaction` is built — no material, and none of the
    /// shading frame a surface whose only role is to be in the way will never need.
    ///
    /// Deliberately unordered. Ordering exists to reach the *nearest* hit sooner; here any hit is
    /// as good as any other, and sorting children would be work spent on a distinction that does
    /// not matter.
    ///
    /// This is the query shadow rays want, and `far` is what carries the light's distance: an
    /// occluder beyond the light is not an occluder.
    pub fn intersect_p(&self, ray: &Ray, near: f64, far: f64) -> bool {
        self.any_hit(ray, near, far, &mut TraversalStats::default())
    }

    /// Same as [`Scene::intersect_p`], but adds the work done to `stats`.
    pub fn intersect_p_instrumented(&self, ray: &Ray, near: f64, far: f64, stats: &mut TraversalStats) -> bool {
        self.any_hit(ray, near, far, stats)
    }

    /// Number of primitives the accelerator was built over, or zero for an empty scene.
    pub fn primitive_count(&self) -> usize {
        self.bvh.as_ref().map_or(0, |bvh_node| bvh_node.primitive_count())
    }

    fn any_hit(&self, ray: &Ray, near: f64, far: f64, stats: &mut TraversalStats) -> bool {
        // The unbounded primitives first: they are the cheap half of the question, and an occluder
        // found there spares the whole tree traversal.
        if Self::any_blocks(self.unbounded.iter(), ray, near, far, stats) {
            return true;
        }

        match &self.bvh {
            None => false,
            Some(bvh_node) => {
                let mut accumulator = ObjectAccumulator { acc: Vec::new() };
                bvh_node.query_instrumented(ray, near, far, &mut accumulator, stats);
                Self::any_blocks(accumulator.acc.iter(), ray, near, far, stats)
            }
        }
    }

    /// Whether any of `primitives` stands in the way.
    ///
    /// `any` short-circuits, so nothing after the first occluder is tested.
    /// `Intersectable::intersect_p` rather than `Object::intersect`: geometry is the whole
    /// question, the material would only be cloned to be dropped, and the primitive is free to
    /// answer without searching for its own nearest hit.
    fn any_blocks<'a>(primitives: impl Iterator<Item = &'a Wrapper<dyn Object>>, ray: &Ray, near: f64, far: f64, stats: &mut TraversalStats) -> bool {
        primitives.into_iter().any(|primitive| {
            stats.object_tests += 1;
            Intersectable::intersect_p(primitive, ray, near, far)
        })
    }

    fn find_nearest(&self, ray: &Ray, near: f64, far: f64, stats: &mut TraversalStats) -> Option<Interaction> {
        // The unbounded primitives are not in the tree, so they are asked directly, every time.
        let mut nearest = Self::nearest_among(self.unbounded.iter(), ray, near, far, stats);

        if let Some(bvh_node) = &self.bvh {
            let mut accumulator = ObjectAccumulator { acc: Vec::new() };
            bvh_node.query_instrumented(ray, near, far, &mut accumulator, stats);

            let from_tree = Self::nearest_among(accumulator.acc.iter(), ray, near, far, stats);
            nearest = Self::closer(nearest, from_tree);
        }

        nearest
    }

    fn nearest_among<'a>(
        primitives: impl Iterator<Item = &'a Wrapper<dyn Object>>,
        ray: &Ray,
        near: f64,
        far: f64,
        stats: &mut TraversalStats,
    ) -> Option<Interaction> {
        primitives.fold(None, |nearest, primitive| {
            stats.object_tests += 1;
            Self::closer(nearest, Object::intersect(primitive, ray, near, far))
        })
    }

    /// Of two candidate interactions, the one the ray reaches first.
    fn closer(a: Option<Interaction>, b: Option<Interaction>) -> Option<Interaction> {
        match (a, b) {
            (None, b) => b,
            (a, None) => a,
            (Some(a), Some(b)) => {
                if b.intersection.d < a.intersection.d {
                    Some(b)
                }
                else {
                    Some(a)
                }
            }
        }
    }

    pub fn get_light_count(&self) -> usize {
        self.lights.len()
    }

    pub fn get_light_at<'a>(&'a self, index: usize) -> Option<&'a dyn Light> {
        if index < self.lights.len() {
            Some(self.lights[index].as_ref())
        }
        else {
            None
        }
    }

    pub fn query_lights<'a>(&'a self, light_type: &LightType) -> Vec<&'a dyn Light> {
        self.lights
            .iter()
            .filter(|light| light.light_type() == *light_type)
            .map(|light| light.as_ref())
            .collect()
    }

    /// Builds the acceleration structure over `prim`, or `None` when there is nothing to
    /// accelerate.
    ///
    /// An empty scene has no tree, and that is how emptiness is represented here: `Option`
    /// already carries it, so `BVHNode` never has to. `intersect` reads `None` as "nothing to
    /// hit" and returns without a single box test.
    fn build_bvh(prim: &mut Vec<Wrapper<dyn Object>>) -> Option<BVHNode<Wrapper<dyn Object>>> {
        if prim.is_empty() {
            return None;
        }

        Some(BVHNode::new(prim))
    }
}

struct ObjectAccumulator {
    pub acc: Vec<Wrapper<dyn Object>>,
}

impl Accumulator<Wrapper<dyn Object>> for ObjectAccumulator {
    fn accumulate(&mut self, items: &mut Vec<Wrapper<dyn Object>>) -> () {
        self.acc.append(items);
        ()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::transform::Transform;
    use crate::geom::vector3::Vector3f;
    use crate::objects::{Simple, Transformed};
    use crate::shapes::{Plane, Sphere};

    /// A scene with no object has no tree, and `intersect` must say so.
    ///
    /// This did not use to fail, it aborted: `commit` handed an empty vector to `BVHNode::new`,
    /// whose median split left the left half empty and recursed on it until the stack overflowed.
    /// Verified by removing the guard — `fatal runtime error: stack overflow`, SIGABRT.
    #[test]
    fn test_empty_scene_intersects_nothing() {
        let mut scene = Scene::new();
        scene.commit();

        let ray = Ray::new(&Vector3f::zero(), &Vector3f::new(0.0, 0.0, 1.0));

        assert!(scene.intersect(&ray, 0.0001, 1000.0).is_none());
    }

    fn lambertian() -> Arc<dyn crate::materials::Material> {
        Arc::new(crate::materials::Lambertian::new(Arc::new(crate::textures::PlainColor::new(
            crate::colors::WHITE,
        ))))
    }

    /// An unbounded primitive stays out of the accelerator, and is still hit.
    ///
    /// The second half is the one that matters. Moving a primitive out of the tree is only sound
    /// if the scene keeps asking it directly, and the failure mode of forgetting to is silent:
    /// the plane would simply stop existing, with no error anywhere.
    #[test]
    fn test_unbounded_primitive_is_kept_out_of_the_tree_but_still_hit() {
        let mut scene = Scene::new();
        scene.add_object(Arc::new(Simple::new(Arc::new(Plane::new()), lambertian())));
        scene.add_object(Arc::new(Simple::new(Arc::new(Sphere::new(1.0)), lambertian())));
        scene.commit();

        assert_eq!(scene.primitive_count(), 1, "only the sphere belongs in the tree");

        // Straight down at the plane, well away from the sphere at the origin.
        let at_the_plane = Ray::new(&Vector3f::new(20.0, 5.0, 20.0), &Vector3f::new(0.0, -1.0, 0.0));
        let hit = scene.intersect(&at_the_plane, 0.0001, 1000.0);
        assert!(hit.is_some(), "the plane is outside the tree, not outside the scene");
        assert!((hit.unwrap().intersection.d - 5.0).abs() < 1e-9);

        // And it occludes, which is the same question asked of the other traversal.
        assert!(scene.intersect_p(&at_the_plane, 0.0001, 1000.0));
    }

    /// The nearest hit is found across both groups, whichever one it belongs to.
    ///
    /// The sphere is pushed *below* the plane on purpose. With it above, the tree would hold the
    /// nearer candidate and the test would pass even if the unbounded list were ignored
    /// altogether — which is exactly what the first version of this test did, and it proved
    /// nothing that its name claimed.
    #[test]
    fn test_nearest_is_found_across_both_groups() {
        let below = Box::new(Transform::translation(Vector3f::new(0.0, -5.0, 0.0)));
        let sphere = Arc::new(Simple::new(Arc::new(Sphere::new(1.0)), lambertian()));

        let mut scene = Scene::new();
        scene.add_object(Arc::new(Simple::new(Arc::new(Plane::new()), lambertian())));
        scene.add_object(Arc::new(Transformed::new(sphere, below)));
        scene.commit();

        assert_eq!(scene.primitive_count(), 1, "only the sphere belongs in the tree");

        // Straight down: the plane at y = 0 comes first, at 5, then the sphere at 9.
        let through_both = Ray::new(&Vector3f::new(0.0, 5.0, 0.0), &Vector3f::new(0.0, -1.0, 0.0));
        let hit = scene.intersect(&through_both, 0.0001, 1000.0).expect("both stand in the way");

        assert!(
            (hit.intersection.d - 5.0).abs() < 1e-9,
            "expected the plane at 5.0, got {}",
            hit.intersection.d
        );
    }
}
