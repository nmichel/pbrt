use std::cmp::Ordering;
use std::fmt;
use std::sync::Arc;

use crate::geom::aabound::{AABound, AABoundingBox};
use crate::geom::intersectable::{Intersectable, Intersection};
use crate::geom::ray::Ray;
use crate::interaction::Interaction;
use crate::materials::Material;
use crate::primitives::Primitive;
use crate::utils::random_double;

pub struct BVHNode<T>
where
    T: Intersectable + AABound,
{
    left: Option<Box<BVHNode<T>>>,
    right: Option<Box<BVHNode<T>>>,
    aabbox: AABoundingBox,
    primitives: Vec<Arc<T>>,
}

pub trait Accumulator<T> {
    fn accumulate(&mut self, item: &Arc<T>, intersections: Vec<Intersection>) -> ();
}

impl<T: Intersectable + AABound> BVHNode<T> {
    pub fn new(primitives: &mut Vec<Arc<T>>) -> Self {
        // Sort primitive with respect to a comparator randomly chosen
        primitives.sort_by(Self::choose_comparator());

        // Depending on the count of elements in vector of primitive
        // Either create BVHNode childrne, or move the primitive
        // in the current node.
        match &primitives[..] {
            // One element, move primitive into current node
            [elem] => {
                Self {
                    left: None,
                    right: None,
                    aabbox: elem.get_bounding_box(),
                    primitives: primitives[..].to_vec(),
                }
            }
            // Several elements, split them among 2 sub nodes
            _ => {
                let half_len = primitives.len() / 2;
                let mut right_half: Vec<_> = primitives.drain(half_len..).collect();
                let left_node = BVHNode::new(primitives);
                let right_node = BVHNode::new(&mut right_half);
                Self {
                    aabbox: AABoundingBox::combine(&left_node.aabbox, &right_node.aabbox),
                    left: Some(Box::new(left_node)),
                    right: Some(Box::new(right_node)),
                    primitives: vec![],
                }
            }
        }
    }

    pub fn intersect(&self, ray: &Ray, near: f64, far: f64, accumulator: &mut dyn Accumulator<T>) -> () {
        if !self.aabbox.hit(ray, near, far) {
            return;
        }

        if self.primitives.len() > 0 {
            self.intersect_local(ray, near, far, accumulator);
        }
        else {
            Self::intersect_subnode(&self.left, ray, near, far, accumulator);
            Self::intersect_subnode(&self.right, ray, near, far, accumulator);
        }
    }

    fn intersect_subnode<'a>(node: &'a Option<Box<BVHNode<T>>>, ray: &Ray, near: f64, far: f64, accumulator: &mut dyn Accumulator<T>) -> () {
        match &node {
            None => (),
            Some(node) => node.intersect(ray, near, far, accumulator),
        }
    }

    fn intersect_local(&self, ray: &Ray, near: f64, far: f64, accumulator: &mut dyn Accumulator<T>) -> () {
        // Use a simple iteration
        let res: Option<Interaction> = None;
        self.primitives.iter().fold(res, |acc, primitive| {
            let intersections = primitive.intersect(ray, near, far);
            accumulator.accumulate(primitive, intersections);
            acc
        });
        ()
    }

    fn choose_comparator() -> fn(&Arc<T>, &Arc<T>) -> Ordering {
        let r = random_double() * 3.0;
        if r < 1.0 {
            Self::compare_x
        }
        else if r < 2.0 {
            Self::compare_y
        }
        else {
            Self::compare_z
        }
    }

    fn compare_x(a: &Arc<T>, b: &Arc<T>) -> Ordering {
        a.get_bounding_box().bmin.x.partial_cmp(&b.get_bounding_box().bmin.x).unwrap()
    }

    fn compare_y(a: &Arc<T>, b: &Arc<T>) -> Ordering {
        a.get_bounding_box().bmin.y.partial_cmp(&b.get_bounding_box().bmin.y).unwrap()
    }

    fn compare_z(a: &Arc<T>, b: &Arc<T>) -> Ordering {
        a.get_bounding_box().bmin.z.partial_cmp(&b.get_bounding_box().bmin.z).unwrap()
    }
}

impl fmt::Display for BVHNode<Primitive> {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "* aabbox {:?}", &self.aabbox)?;
        writeln!(f, "primitives {}", self.primitives.len())?;
        match &self.left {
            None => {
                writeln!(f, "No left child")?;
            }
            Some(sub_node) => {
                writeln!(f, "left child \n{}", *sub_node)?;
            }
        }
        match &self.right {
            None => {
                writeln!(f, "No right child")?;
            }
            Some(sub_node) => {
                writeln!(f, "right child \n{}", *sub_node)?;
            }
        }

        fmt::Result::Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use crate::colors;
    use crate::geom::transform::Transform;
    use crate::geom::vector3::Vector3f;
    use crate::materials::{Lambertian, Material};
    use crate::shapes::Sphere;
    use crate::textures::PlainColor;

    use super::*;

    #[test]
    fn test_bvh_build() {
        let material: Arc<dyn Material> = Arc::new(Lambertian::new(Arc::new(PlainColor::new(colors::ORANGE))));
        let mut prims = vec![
            Arc::new(Primitive::new(
                Box::new(Sphere::new(1.0)),
                Box::new(Transform::translation(Vector3f::new(0.0, 0.0, 0.0))),
                Arc::clone(&material),
            )),
            Arc::new(Primitive::new(
                Box::new(Sphere::new(2.0)),
                Box::new(Transform::translation(Vector3f::new(-2.0, 0.0, 0.0))),
                Arc::clone(&material),
            )),
        ];
        let bvh = BVHNode::new(&mut prims);
        println!("Node \n{}", &bvh);
    }
}
