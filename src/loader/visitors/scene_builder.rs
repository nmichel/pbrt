use std::sync::Arc;

use super::super::ast::*;
use super::super::mesh_loader::load_ply_mesh;
use super::Visitor;
use crate::cameras::{Camera, PinHoleCamera, ThinLensCamera};
use crate::config::Config;
use crate::geom::matrix4::Matrix4;
use crate::geom::transform::Transform;
use crate::geom::vector2::Vector2u;
use crate::lights::{AreaLight, BackgroundInfiniteLight, Light, PointLight, UniformInfiniteLight};
use crate::materials::*;
use crate::objects::*;
use crate::scene::Scene;
use crate::shapes::{self, *};
use crate::textures::*;

pub struct SceneBuilderVisitor<'a> {
    pub scene: Scene,
    pub camera: Option<Box<dyn Camera>>,
    csg_elems: Vec<Arc<dyn Shape>>,
    lights: Vec<Arc<dyn Light>>,

    /// The lights an emissive object *is*, as opposed to the ones a `light` production declares.
    ///
    /// They are held apart because the two are counted differently: `visit_scene` knows how many
    /// lights the description declared and takes exactly that many off `lights`, while these arrive
    /// from wherever an `object simple` was visited — nested inside a compound or a placement as
    /// well as at the top — and are known only by having been collected.
    area_lights: Vec<Arc<dyn Light>>,

    objects: Vec<Arc<dyn Object>>,
    shapes: Vec<Arc<dyn Shape>>,
    materials: Vec<Arc<dyn Material>>,
    textures: Vec<Arc<dyn Texture>>,
    transforms: Vec<Box<Transform>>,

    /// The current transformation matrix: where the frame being described sits in the world.
    ///
    /// One entry per enclosing `object transformed` block, each already composed with the one
    /// below it, so that the top is the whole placement and `place` needs no fold. Pushed when a
    /// block opens, dropped when it closes; empty means the description is in world coordinates
    /// already.
    ctm: Vec<Box<Transform>>,
    config: &'a Config,
}

impl<'a> SceneBuilderVisitor<'a> {
    pub fn new(config: &'a Config) -> Self {
        SceneBuilderVisitor {
            scene: Scene::new(),
            camera: None,
            csg_elems: Vec::new(),
            lights: Vec::new(),
            area_lights: Vec::new(),
            objects: Vec::new(),
            shapes: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
            transforms: Vec::new(),
            ctm: Vec::new(),
            config,
        }
    }

    pub fn visit(self: &mut Self, node: &StageNode) {
        node.visit(self);
    }

    /// `shape` in world space: the current transformation matrix, folded into the geometry.
    ///
    /// Outside any `transform` block the shape is handed back **untouched** rather than wrapped in
    /// an identity placement. That is not a micro-optimisation: `Transform::transform_ray_to_local`
    /// builds its ray through `Ray::new`, which renormalises, and dividing a unit vector by its own
    /// computed length is not the identity in floating point. An object nobody placed must not
    /// move, so it gets no placement at all.
    fn place(&self, shape: Arc<dyn Shape>) -> Arc<dyn Shape> {
        match self.ctm.last() {
            None => shape,
            // Each placed shape owns its copy of the matrix: a `Transform` is a value, and the
            // siblings of a compound need the same one after this leaf has taken it.
            Some(ctm) => Arc::new(shapes::Transformed::new(shape, Box::new((**ctm).clone()))),
        }
    }

    /// The `count` elements of an assembly, taken off the stack.
    ///
    /// They come back **in reverse order of declaration**, a stack handing back the last one
    /// first. A union and an intersection are commutative, so the order is theirs to ignore; a
    /// substraction is not, its first element being the base the others are removed from. So
    /// `csg substraction { elem A … elem B … }` removes A from B, while the programmatic
    /// constructions in `examples/` pass their elements in reading order and remove B from A. The
    /// two disagree, and `IDEAS.md` holds the question under *Renderer & infrastructure*.
    fn pop_csg_elems(&mut self, count: usize) -> Vec<Arc<dyn Shape>> {
        let mut elems: Vec<Arc<dyn Shape>> = Vec::new();
        for _ in 1..=count {
            elems.push(self.csg_elems.pop().unwrap());
        }
        elems
    }
}

impl Visitor for SceneBuilderVisitor<'_> {
    fn visit_stage(self: &mut Self, _node: &StageNode) {
        // Nothing special for now
    }

    /// Lights enter the scene **in declaration order**, objects in the reverse of it.
    ///
    /// The asymmetry is not an oversight. A light's index is a coordinate:
    /// [`PathIntegrator`](crate::integrators::PathIntegrator) picks one by drawing a number and
    /// scaling it to the count, so reversing the list hands the same draw a different source and
    /// the same seed a different image. An object's index is read by nothing — the accelerator
    /// sorts the primitives itself.
    ///
    /// The lights emissive objects *are* come last, after every declared one. Nothing forces that
    /// order, and the choice is the one that moves the fewest indices: a declared light keeps the
    /// position its file gives it whatever objects the scene also holds.
    fn visit_scene(self: &mut Self, node: &SceneNode) {
        let first_declared = self.lights.len() - node.lights.len();
        let lights: Vec<Arc<dyn Light>> = self.lights.drain(first_declared..).collect();
        for light in lights {
            self.scene.add_light(light);
        }

        let area_lights: Vec<Arc<dyn Light>> = self.area_lights.drain(..).collect();
        for light in area_lights {
            self.scene.add_light(light);
        }

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

    /// The leaf of the description, and the one place a placement is now paid for.
    ///
    /// The shape comes out in world space, so the object holds geometry that needs nothing done to
    /// it afterwards — and the area light sampling that surface reads its points from **the same**
    /// `Arc<dyn Shape>`. There is nothing to keep in agreement, because there is one geometry: a
    /// light built from its own copy could drift from the object, and the shadow it cast would be
    /// the only sign of it.
    ///
    /// # Two questions, and the diagnostic is their conjunction
    ///
    /// Does this material emit, and can this shape hand out points of itself? Each is asked of the
    /// thing itself — [`Material::emitter`](crate::materials::Material::emitter) and
    /// [`Shape::area_sampler`] — and each answers `None` by default, so a new material or a new
    /// shape joins in without this `match` growing an arm.
    ///
    /// Three of the four combinations are ordinary. The fourth — it emits, it cannot be sampled —
    /// is a description that cannot be rendered as it asks: the surface would be seen and would
    /// light nothing, which is precisely the defect an area light exists to remove. Rendering it
    /// anyway would reinstate that defect silently for one object, so the load stops and names the
    /// shape. The diagnostic is not an extra check; it is the arm no other arm covers.
    fn visit_object_simple(self: &mut Self, node: &ObjectSimpleNode) {
        let material = self.materials.pop().unwrap();
        let local_shape = self.shapes.pop().unwrap();
        let shape = self.place(local_shape);

        self.objects.push(Arc::new(Simple::new(Arc::clone(&shape), Arc::clone(&material))));

        match (Arc::clone(&material).emitter(), Arc::clone(&shape).area_sampler()) {
            (Some(emitter), Some(sampler)) => self.area_lights.push(Arc::new(AreaLight::new(sampler, emitter))),
            (Some(_), None) => {
                panic!(
                    "an emissive material sits on a `{}`, which cannot hand out points of itself: it would be seen and would light nothing",
                    node.shape.name()
                )
            }
            (None, _) => {}
        }
    }

    /// Composes the block's transformation into the current transformation matrix.
    ///
    /// Taking the transformation off `transforms` here, rather than reading it at the end, is also
    /// what keeps it out of reach of the geometry below: a `csg elem` pops that same stack, and
    /// would otherwise find an object's placement sitting where its own belongs.
    fn enter_object_transformed(self: &mut Self, _node: &ObjectTransformedNode) {
        let to_world = self.transforms.pop().unwrap();

        // p_world = enclosing ⋅ (this block ⋅ p_local), so the composition is in that order. With
        // nothing enclosing, the block's own transformation *is* the matrix, and is taken as it
        // stands: multiplying by an identity would be exact, but there is nothing to multiply.
        let composed = match self.ctm.last() {
            None => to_world,
            Some(enclosing) => Box::new(&**enclosing * &*to_world),
        };
        self.ctm.push(composed);
    }

    /// Drops the block's placement, the geometry underneath having been built with it.
    ///
    /// Nothing is wrapped around the object: it is already where the description put it. This is
    /// the whole difference the current transformation matrix makes, and the reason
    /// [`objects::Transformed`](crate::objects::Transformed) is no longer built here — in the
    /// loader that type is the *instancing* mechanism, waiting for a caller that can name a
    /// structure and repeat it.
    fn leave_object_transformed(self: &mut Self, _node: &ObjectTransformedNode) {
        self.ctm.pop();
    }

    /// A light placed by its own `transform` block, and by nothing else.
    ///
    /// The transformation is taken from the same stack a `csg elem` reads, and **not** composed
    /// with the current transformation matrix: the grammar only accepts a light directly inside
    /// `scene`, so there is never an enclosing `object transformed` whose placement could apply.
    /// The position a light writes is the position it gets.
    fn visit_light_point(self: &mut Self, node: &PointLightNode) {
        let to_world = self.transforms.pop().unwrap();
        self.lights.push(Arc::new(PointLight::new(to_world, node.intensity)));
    }

    fn visit_light_uniform_infinite(self: &mut Self, node: &UniformInfiniteLightNode) {
        self.lights.push(Arc::new(UniformInfiniteLight::new(node.radiance)));
    }

    fn visit_light_background_infinite(self: &mut Self, node: &BackgroundInfiniteLightNode) {
        self.lights.push(Arc::new(BackgroundInfiniteLight::new(node.bottom, node.top)));
    }

    fn visit_shape_aabox(self: &mut Self, node: &AABoxShapeNode) {
        self.shapes.push(Arc::new(AABox::new(&node.extend)));
    }

    /// A piece of an assembly: a shape placed in the assembly's own frame.
    ///
    /// This transformation does **not** go through the current transformation matrix, and the entry
    /// hook of `object transformed` is what keeps the two apart. It positions a piece inside the
    /// assembly, where the matrix positions the whole assembly in the world; composing them would
    /// place the assembly twice ([`csg`](crate::shapes::csg) header).
    fn visit_shape_csg_elem(self: &mut Self, _node: &CSGShapeElemNode) {
        let to_world = self.transforms.pop().unwrap();
        let shape = self.shapes.pop().unwrap();
        self.csg_elems.push(Arc::new(shapes::Transformed::new(shape, to_world)));
    }

    fn visit_shape_csg_intersection(self: &mut Self, node: &CSGShapeIntersectionNode) {
        let elems = self.pop_csg_elems(node.elems.len());
        self.shapes.push(Arc::new(csg::Intersection::new(elems)));
    }

    fn visit_shape_csg_substraction(self: &mut Self, node: &CSGShapeSubstractionNode) {
        let elems = self.pop_csg_elems(node.elems.len());
        self.shapes.push(Arc::new(csg::Substraction::new(elems)));
    }

    fn visit_shape_csg_union(self: &mut Self, node: &CSGShapeUnionNode) {
        let elems = self.pop_csg_elems(node.elems.len());
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
        self.shapes.push(Arc::new(load_ply_mesh(&node.filename, node.reverse)));
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::default_config;
    use crate::geom::intersectable::Intersection;
    use crate::geom::ray::Ray;
    use crate::geom::vector2::Vector2u;
    use crate::geom::vector3::Vector3f;
    use crate::lights::LightType;
    use crate::loader::parser::Parser;
    use crate::samplers::IndependentSampler;

    /// A shaded point at the world origin, with only the fields a light reads filled in.
    fn shaded_origin() -> Intersection {
        Intersection {
            p: Vector3f::new(0.0, 0.0, 0.0),
            d: 0.0,
            n: Vector3f::new(0.0, 1.0, 0.0),
            wo: Vector3f::new(0.0, 1.0, 0.0),
            u: 0.0,
            v: 0.0,
            dpdu: Vector3f::new(1.0, 0.0, 0.0),
            dpdv: Vector3f::new(0.0, 0.0, 1.0),
        }
    }

    /// What a description declares is what the scene holds, in the order it was written.
    ///
    /// Every visitor method here pops from a stack, so a light built from the wrong entry — or
    /// one left behind on it — is the mistake to catch. Printing cannot see it: the text is
    /// produced from the node, the scene from the stack.
    #[test]
    fn test_a_scene_holds_the_lights_its_description_declares() {
        let input = "
    scene
      light point
        color 15.0 15.0 15.0
        transform {
            translate 0.0 2.0 1.0
        }

      light uniform_infinite
        color 0.5 0.5 0.5

      light background_infinite
        color 1.0 1.0 1.0
        color 0.5 0.7 1.0
    ";

        let config = default_config();
        let mut visitor = SceneBuilderVisitor::new(&config);
        Parser::new(input).parse_scene().visit(&mut visitor);

        assert_eq!(visitor.scene.get_light_count(), 3);
        assert_eq!(visitor.scene.get_light_at(0).unwrap().light_type(), LightType::Point);
        assert_eq!(visitor.scene.get_light_at(1).unwrap().light_type(), LightType::Infinite);
        assert_eq!(visitor.scene.get_light_at(2).unwrap().light_type(), LightType::Infinite);
    }

    /// A point light is placed by its own block, and the placement reaches the light itself.
    ///
    /// `sample_li` from the origin must aim at the declared position: the transformation stack a
    /// `csg elem` also reads is shared, so a light taking the wrong entry would sit somewhere
    /// else entirely — and nothing in the printed text would say so.
    #[test]
    fn test_a_point_light_sits_where_its_transform_puts_it() {
        let input = "
    scene
      light point
        color 15.0 15.0 15.0
        transform {
            translate 0.0 2.0 1.0
        }
    ";

        let config = default_config();
        let mut visitor = SceneBuilderVisitor::new(&config);
        Parser::new(input).parse_scene().visit(&mut visitor);

        let mut sampler = IndependentSampler::new(0, &Vector2u::new(0, 0), 0);
        let light = visitor.scene.get_light_at(0).unwrap();
        let (sample, _) = light.sample_li(&shaded_origin(), &mut sampler).unwrap();

        // The direction to (0, 2, 1) seen from the origin, normalised.
        let expected = Vector3f::new(0.0, 2.0, 1.0).normalized();
        assert!((sample.wi - expected).length() < 1e-12);
    }

    /// A lamp hanging over the origin, facing down, declared the way a scene declares one.
    ///
    /// The placement is what the test is about, so it is never the identity: the rectangle is
    /// authored at the origin and carried to y = 4, which is exactly the step a light sampling the
    /// unplaced shape would skip.
    fn lamp_over_the_origin(material: &str, shape: &str) -> String {
        format!(
            "
    scene
      object transformed
        object simple
          {shape}
          {material}
        transform {{
          rotate_x 3.14159265
          translate 0.0 4.0 0.0
        }}
    "
        )
    }

    fn build(input: &str) -> SceneBuilderVisitor<'static> {
        // The configuration outlives the visitor, which borrows it; a test scene needs nothing of
        // it but its defaults.
        let config: &'static _ = Box::leak(Box::new(default_config()));
        let mut visitor = SceneBuilderVisitor::new(config);
        Parser::new(input).parse_scene().visit(&mut visitor);
        visitor.scene.commit();
        visitor
    }

    /// An object whose material emits is also a light, without being declared as one.
    ///
    /// This is the whole of the registration: the description says `object`, and the scene holds
    /// both a primitive a ray can hit and a source next event estimation can aim at.
    #[test]
    fn test_an_emissive_object_is_a_light_of_the_scene() {
        let visitor = build(&lamp_over_the_origin("diffuse_light color 5.0 5.0 5.0", "rectangle 2.0 2.0"));

        assert_eq!(visitor.scene.get_light_count(), 1);
        assert_eq!(visitor.scene.get_light_at(0).unwrap().light_type(), LightType::Area);
    }

    /// A material that does not emit leaves the scene with no light at all.
    ///
    /// The pair with the test above is what says the registration reads the *material*, rather
    /// than registering every shape that happens to know how to sample itself.
    #[test]
    fn test_an_object_that_does_not_emit_is_not_a_light() {
        let visitor = build(&lamp_over_the_origin("lambertian color 0.5 0.5 0.5", "rectangle 2.0 2.0"));

        assert_eq!(visitor.scene.get_light_count(), 0);
    }

    /// The light samples the **placed** surface, which is the one the object renders.
    ///
    /// A point drawn on the source is checked by shooting a ray at it: the ray must find the lamp
    /// where the description hung it. Reading the light's own geometry would prove nothing — the
    /// question is whether the two participants look at the same `Arc`, and the accelerator holds
    /// the object's side of it. A light built from the unplaced rectangle would sit at y = 0,
    /// through the shaded point itself.
    #[test]
    fn test_the_light_and_the_object_are_the_same_surface() {
        let visitor = build(&lamp_over_the_origin("diffuse_light color 5.0 5.0 5.0", "rectangle 2.0 2.0"));

        let mut sampler = IndependentSampler::new(0, &Vector2u::new(0, 0), 0);
        let light = visitor.scene.get_light_at(0).unwrap();

        // The ray crosses the placement twice — into the shape's own frame and back — so the point
        // it reports carries a few rounding errors. What is being told apart is y = 4 from y = 0,
        // and any bound far below the placement itself settles that.
        let tolerance = 1e-6;

        for _ in 0..64 {
            let (sample, _) = light.sample_li(&shaded_origin(), &mut sampler).unwrap();

            let towards_light = Ray::new(&shaded_origin().p, &sample.wi);
            let hit = visitor.scene.intersect(&towards_light, 0.0001, 100.0).unwrap().intersection;

            assert!((hit.p.y - 4.0).abs() < tolerance, "the lamp is at y = 4, the ray found {}", hit.p.y);
            assert!(
                hit.p.x.abs() <= 1.0 + tolerance && hit.p.z.abs() <= 1.0 + tolerance,
                "outside the lamp: {:?}",
                hit.p
            );
        }
    }

    /// Declared lights keep the index their file gives them, whatever objects the scene holds.
    ///
    /// The index is a coordinate — `sample_light` scales a draw to the count — so this is what
    /// keeps a scene's declared sources where they were when an emissive object joins them.
    #[test]
    fn test_an_emissive_object_comes_after_every_declared_light() {
        let input = "
    scene
      light point
        color 15.0 15.0 15.0
        transform {
            translate 0.0 2.0 1.0
        }

      object transformed
        object simple
          rectangle 2.0 2.0
          diffuse_light color 5.0 5.0 5.0
        transform {
          rotate_x 3.14159265
          translate 0.0 4.0 0.0
        }
    ";

        let visitor = build(input);

        assert_eq!(visitor.scene.get_light_count(), 2);
        assert_eq!(visitor.scene.get_light_at(0).unwrap().light_type(), LightType::Point);
        assert_eq!(visitor.scene.get_light_at(1).unwrap().light_type(), LightType::Area);
    }

    /// Emission on a shape that cannot hand out points of itself stops the load.
    ///
    /// A `plane` is unbounded and has no area to draw from. Rendering the description anyway would
    /// show the surface and light nothing with it — the very defect an area light removes, kept
    /// alive silently for one object. The message quotes the word the description used.
    #[test]
    #[should_panic(expected = "`plane`")]
    fn test_emission_on_a_shape_that_cannot_be_sampled_stops_the_load() {
        build(&lamp_over_the_origin("diffuse_light color 5.0 5.0 5.0", "plane"));
    }
}
