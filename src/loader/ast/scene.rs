use crate::loader::visitors::Visitor;

use super::{LightNode, Node, ObjectNode};

/// What a scene is made of: things that are seen, and things that light them.
///
/// The two are **siblings**, and deliberately so. A light with no geometry is a member of the
/// scene, not of an object: routing it through `object` would send it down the shape-plus-material
/// path, which is exactly what it does not have.
///
/// The price of the split is that the two lists no longer remember how they were interleaved in
/// the file. A description mixing `light` and `object` therefore reads back with its lights
/// gathered in front — the same scene, written in a normal form.
pub struct SceneNode {
    pub lights: Vec<Box<dyn LightNode>>,
    pub objects: Vec<Box<dyn ObjectNode>>,
}

impl SceneNode {
    pub fn new() -> Self {
        SceneNode {
            lights: Vec::new(),
            objects: Vec::new(),
        }
    }

    pub fn add_light(self: &mut Self, light: Box<dyn LightNode>) {
        self.lights.push(light);
    }

    pub fn add_object(self: &mut Self, object: Box<dyn ObjectNode>) {
        self.objects.push(object);
    }
}

impl Node for SceneNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        for light in &self.lights {
            light.visit(visitor);
        }
        for object in &self.objects {
            object.visit(visitor);
        }
        visitor.visit_scene(self);
    }
}
