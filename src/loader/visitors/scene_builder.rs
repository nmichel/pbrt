use std::sync::Arc;

use super::super::ast::*;
use super::Visitor;
use crate::geom::transform::Transform;
use crate::materials::*;
use crate::objects::*;
use crate::scene::Scene;
use crate::shapes::*;
use crate::textures::*;

pub struct SceneBuilderVisitor {
    pub scene: Scene,
    csg_elems: Vec<Box<csg::Elem>>,
    objects: Vec<Arc<dyn Object>>,
    shapes: Vec<Arc<dyn Shape>>,
    materials: Vec<Arc<dyn Material>>,
    textures: Vec<Arc<dyn Texture>>,
    transforms: Vec<Box<Transform>>,
}

impl SceneBuilderVisitor {
    pub fn new() -> Self {
        SceneBuilderVisitor {
            scene: Scene::new(),
            csg_elems: Vec::new(),
            objects: Vec::new(),
            shapes: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
            transforms: Vec::new(),
        }
    }

    pub fn visit(self: &mut Self, node: &SceneNode) {
        node.visit(self);
    }
}

impl Visitor for SceneBuilderVisitor {
    fn visit_scene(self: &mut Self, node: &SceneNode) {
        for _ in 1..=node.objects.len() {
            let object = self.objects.pop().unwrap();
            self.scene.add_object(object);
        }
    }

    fn visit_object_compound(self: &mut Self, node: &ObjectCompoundNode) {
        let mut objects: Vec<Arc<dyn Object>> = Vec::new();
        for _ in 1..=node.objects.len() {
            objects.push(self.objects.pop().unwrap());
        }

        let compound = Arc::new(Compound::new(&objects));
        self.objects.push(compound);
    }

    fn visit_object_simple(self: &mut Self, _node: &ObjectSimpleNode) {
        let material = self.materials.pop().unwrap();
        let shape = self.shapes.pop().unwrap();
        self.objects.push(Arc::new(Simple::new(shape, material)));
    }

    fn visit_object_transformed(self: &mut Self, _node: &ObjectTransformedNode) {
        let object = self.objects.pop().unwrap();
        let transform = self.transforms.pop().unwrap();
        self.objects.push(Arc::new(Transformed::new(object, transform)));
    }

    fn visit_shape_aabox(self: &mut Self, node: &AABoxShapeNode) {
        self.shapes.push(Arc::new(AABox::new(&node.extend)));
    }

    fn visit_shape_csg_elem(self: &mut Self, _node: &CSGShapeElemNode) {
        let transform = self.transforms.pop().unwrap();
        let shape: Arc<dyn Shape> = self.shapes.pop().unwrap();
        self.csg_elems.push(Box::new(csg::Elem { shape, transform }));
    }

    fn visit_shape_csg_intersection(self: &mut Self, node: &CSGShapeIntersectionNode) {
        let mut elems: Vec<Box<csg::Elem>> = Vec::new();
        for _ in 1..=node.elems.len() {
            elems.push(self.csg_elems.pop().unwrap());
        }
        self.shapes.push(Arc::new(csg::Intersection::new(elems)));
    }

    fn visit_shape_csg_substraction(self: &mut Self, node: &CSGShapeSubstractionNode) {
        let mut elems: Vec<Box<csg::Elem>> = Vec::new();
        for _ in 1..=node.elems.len() {
            elems.push(self.csg_elems.pop().unwrap());
        }
        self.shapes.push(Arc::new(csg::Substraction::new(elems)));
    }

    fn visit_shape_csg_union(self: &mut Self, node: &CSGShapeUnionNode) {
        let mut elems: Vec<Box<csg::Elem>> = Vec::new();
        for _ in 1..=node.elems.len() {
            elems.push(self.csg_elems.pop().unwrap());
        }
        self.shapes.push(Arc::new(csg::Union::new(elems)));
    }

    fn visit_shape_cylinder(self: &mut Self, node: &CylinderShapeNode) {
        self.shapes.push(Arc::new(Cylinder::new(node.radius, node.height)));
    }

    fn visit_shape_plane(self: &mut Self, _node: &PlaneShapeNode) {
        self.shapes.push(Arc::new(Plane::new()));
    }

    fn visit_shape_rectangle(self: &mut Self, node: &RectangleShapeNode) {
        self.shapes.push(Arc::new(Rectangle::new(node.half_width, node.half_height)));
    }

    fn visit_shape_sphere(self: &mut Self, node: &SphereShapeNode) {
        self.shapes.push(Arc::new(Sphere::new(node.radius)));
    }

    fn visit_material_dielectric(self: &mut Self, node: &DielectricMaterialNode) {
        let texture = self.textures.pop().unwrap();
        self.materials.push(Arc::new(Dielectric::new(node.index, texture)));
    }

    fn visit_material_lambertian(self: &mut Self, _node: &LambertianMaterialNode) {
        let texture = self.textures.pop().unwrap();
        self.materials.push(Arc::new(Lambertian::new(texture)));
    }

    fn visit_material_metal(self: &mut Self, node: &MetalMaterialNode) {
        let texture = self.textures.pop().unwrap();
        self.materials.push(Arc::new(Metal::new(node.fuzz, texture)));
    }

    fn visit_texture_checkerboard(self: &mut Self, node: &CheckerboardTextureNode) {
        self.textures.push(Arc::new(CheckerBoard::new(node.color1, node.color2, node.scale)));
    }

    fn visit_texture_color(self: &mut Self, node: &ColorTextureNode) {
        self.textures.push(Arc::new(PlainColor::new(node.color)));
    }

    fn visit_transform(self: &mut Self, node: &TransformNode) {
        let mut transforms: Vec<Box<Transform>> = Vec::new();
        for _ in 1..=node.steps.len() {
            let step = self.transforms.pop().unwrap();
            transforms.push(step);
        }

        let res = transforms.iter().fold(Transform::identity(), |acc, t| {
            let op: &Transform = &**t;
            &acc * op
        });
        self.transforms.push(Box::new(res));
    }

    fn visit_transform_rotate(self: &mut Self, node: &TransformRotateAxisNode) {
        match node.axis {
            Axis::X => self.transforms.push(Box::new(Transform::rotation_x(node.angle))),
            Axis::Y => self.transforms.push(Box::new(Transform::rotation_y(node.angle))),
            Axis::Z => self.transforms.push(Box::new(Transform::rotation_z(node.angle))),
        }
    }

    fn visit_transform_translate(self: &mut Self, node: &TransformTranslateNode) {
        self.transforms.push(Box::new(Transform::translation(node.offset)));
    }
}
