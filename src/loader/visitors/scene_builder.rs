use std::convert::TryFrom;
use std::fmt::Debug;
use std::sync::Arc;

use super::super::ast::*;
use super::super::ply::{read_ply_file, PlyElementDesc, PlyElementProps, PlyEventObserver, PlyPropertyValue};
use super::Visitor;
use crate::cameras::{Camera, PinHoleCamera, ThinLensCamera};
use crate::config::Config;
use crate::geom::matrix4::Matrix4;
use crate::geom::transform::Transform;
use crate::geom::vector2::Vector2u;
use crate::materials::*;
use crate::objects::*;
use crate::scene::Scene;
use crate::shapes::*;
use crate::textures::*;

struct PlyObserver {
    vertices: Vec<f64>,
    faces: Vec<usize>,
    reverse: bool,
}

impl PlyEventObserver for PlyObserver {
    fn on_header_complete(&mut self, _header: &Vec<PlyElementDesc>) {}

    fn on_vertex_start(&mut self) {}

    fn on_vertex_event(self: &mut Self, props: &PlyElementProps, value: PlyPropertyValue) {
        match props {
            PlyElementProps::VertexX | PlyElementProps::VertexY | PlyElementProps::VertexZ => {
                self.vertices.push(f64::try_from(&value).unwrap());
            }
            _ => {}
        }
    }

    fn on_vertex_end(&mut self) {}

    fn on_face_start(&mut self) {}

    fn on_face_event(self: &mut Self, props: &PlyElementProps, value: PlyPropertyValue) {
        fn push_to_faces<T>(list: &[T], faces: &mut Vec<usize>, reverse: bool)
        where
            usize: TryFrom<T>,
            T: Copy,
            <usize as TryFrom<T>>::Error: Debug,
        {
            if reverse {
                for index in list.iter().rev() {
                    faces.push(usize::try_from(*index).unwrap());
                }
            }
            else {
                for index in list.iter() {
                    faces.push(usize::try_from(*index).unwrap());
                }
            }
        }

        match props {
            PlyElementProps::FaceVertexIndices => {
                match value {
                    PlyPropertyValue::ListUInt32(list) => {
                        push_to_faces(&list[..], &mut self.faces, self.reverse);
                    }
                    PlyPropertyValue::ListInt32(list) => {
                        push_to_faces(&list[..], &mut self.faces, self.reverse);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn on_face_end(&mut self) {}

    fn on_data_complete(&mut self) {}
}

pub struct SceneBuilderVisitor<'a> {
    pub scene: Scene,
    pub camera: Option<Box<dyn Camera>>,
    csg_elems: Vec<Box<csg::Elem>>,
    objects: Vec<Arc<dyn Object>>,
    shapes: Vec<Arc<dyn Shape>>,
    materials: Vec<Arc<dyn Material>>,
    textures: Vec<Arc<dyn Texture>>,
    transforms: Vec<Box<Transform>>,
    config: &'a Config,
}

impl<'a> SceneBuilderVisitor<'a> {
    pub fn new(config: &'a Config) -> Self {
        SceneBuilderVisitor {
            scene: Scene::new(),
            camera: None,
            csg_elems: Vec::new(),
            objects: Vec::new(),
            shapes: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
            transforms: Vec::new(),
            config,
        }
    }

    pub fn visit(self: &mut Self, node: &StageNode) {
        node.visit(self);
    }
}

impl Visitor for SceneBuilderVisitor<'_> {
    fn visit_stage(self: &mut Self, _node: &StageNode) {
        // Nothing special for now
    }

    fn visit_scene(self: &mut Self, node: &SceneNode) {
        for _ in 1..=node.objects.len() {
            let object = self.objects.pop().unwrap();
            self.scene.add_object(object);
        }
    }

    fn visit_camera_pin_hole(self: &mut Self, node: &PinHoleCameraNode) {
        let fov = self.config.fov_deg * std::f64::consts::PI / 180.0;
        let resolution = Vector2u::new(self.config.output_width as u32, self.config.output_height as u32);
        let cam_to_world = Matrix4::look_at(&node.pos, &node.look, &node.up);

        self.camera = Some(Box::new(PinHoleCamera::new(
            &resolution,
            fov,
            self.config.near,
            self.config.far,
            cam_to_world,
        )));
    }

    fn visit_camera_thin_lens(self: &mut Self, node: &ThinLensCameraNode) {
        let fov = self.config.fov_deg * std::f64::consts::PI / 180.0;
        let resolution = Vector2u::new(self.config.output_width as u32, self.config.output_height as u32);
        let cam_to_world = Matrix4::look_at(&node.pos, &node.look, &node.up);

        self.camera = Some(Box::new(ThinLensCamera::new(
            &resolution,
            fov,
            self.config.near,
            self.config.far,
            node.radius,
            node.focal_length,
            cam_to_world,
        )));
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

    fn visit_shape_mesh(self: &mut Self, node: &MeshShapeNode) {
        let mut observer = PlyObserver {
            vertices: Vec::new(),
            faces: Vec::new(),
            reverse: node.reverse,
        };

        read_ply_file(&node.filename, &mut observer).unwrap();

        self.shapes
            .push(Arc::new(TriangleMesh::new(observer.vertices, observer.faces, None, None)));
    }

    fn visit_material_dielectric(self: &mut Self, node: &DielectricMaterialNode) {
        let texture = self.textures.pop().unwrap();
        self.materials.push(Arc::new(Dielectric::new(node.index, texture)));
    }

    fn visit_material_diffuse_light(self: &mut Self, _node: &DiffuseLightMaterialNode) {
        let texture = self.textures.pop().unwrap();
        self.materials.push(Arc::new(DiffuseLight::new(texture)));
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
