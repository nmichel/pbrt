use super::ast::*;

pub trait Visitor {
    fn visit_stage(self: &mut Self, node: &StageNode);

    fn visit_scene(self: &mut Self, node: &SceneNode);

    fn visit_camera_pin_hole(self: &mut Self, node: &PinHoleCameraNode);
    fn visit_camera_thin_lens(self: &mut Self, node: &ThinLensCameraNode);

    fn visit_object_compound(self: &mut Self, node: &ObjectCompoundNode);
    fn visit_object_simple(self: &mut Self, node: &ObjectSimpleNode);

    /// Called when a `transform` block opens, its transformation already visited and the object it
    /// places not yet.
    ///
    /// The only pair of hooks in this trait, and the only node that needs one: every other node is
    /// told about its children once they are done, where this one has something to say *before*
    /// them — the placement under which they are to be built. A visitor that carries a current
    /// transformation matrix composes it here and drops it in `leave_object_transformed`.
    ///
    /// **A `csg elem`'s transformation deliberately does not come through here.** It places a piece
    /// *inside* an assembly, in the assembly's own frame, where this one places a whole object in
    /// the world; composing the two would place the assembly twice. That is why the entry hook
    /// belongs to this node rather than to `visit_transform`, which both of them go through.
    fn enter_object_transformed(self: &mut Self, node: &ObjectTransformedNode);

    /// Called when the block closes, the object it places being built.
    fn leave_object_transformed(self: &mut Self, node: &ObjectTransformedNode);

    fn visit_light_point(self: &mut Self, node: &PointLightNode);
    fn visit_light_uniform_infinite(self: &mut Self, node: &UniformInfiniteLightNode);
    fn visit_light_background_infinite(self: &mut Self, node: &BackgroundInfiniteLightNode);

    fn visit_shape_aabox(self: &mut Self, node: &AABoxShapeNode);
    fn visit_shape_csg_elem(self: &mut Self, node: &CSGShapeElemNode);
    fn visit_shape_csg_intersection(self: &mut Self, node: &CSGShapeIntersectionNode);
    fn visit_shape_csg_substraction(self: &mut Self, node: &CSGShapeSubstractionNode);
    fn visit_shape_csg_union(self: &mut Self, node: &CSGShapeUnionNode);
    fn visit_shape_cylinder(self: &mut Self, node: &CylinderShapeNode);
    fn visit_shape_mesh(self: &mut Self, node: &MeshShapeNode);

    fn visit_shape_plane(self: &mut Self, node: &PlaneShapeNode);
    fn visit_shape_rectangle(self: &mut Self, node: &RectangleShapeNode);
    fn visit_shape_sphere(self: &mut Self, node: &SphereShapeNode);

    fn visit_material_dielectric(self: &mut Self, node: &DielectricMaterialNode);
    fn visit_material_diffuse_light(self: &mut Self, node: &DiffuseLightMaterialNode);
    fn visit_material_lambertian(self: &mut Self, node: &LambertianMaterialNode);
    fn visit_material_metal(self: &mut Self, node: &MetalMaterialNode);

    fn visit_texture_checkerboard(self: &mut Self, node: &CheckerboardTextureNode);
    fn visit_texture_color(self: &mut Self, node: &ColorTextureNode);

    fn visit_transform(self: &mut Self, node: &TransformNode);
    fn visit_transform_rotate(self: &mut Self, node: &TransformRotateAxisNode);
    fn visit_transform_translate(self: &mut Self, node: &TransformTranslateNode);
}

mod print;
mod scene_builder;

pub use self::print::PrintVisitor;
pub use self::scene_builder::SceneBuilderVisitor;
