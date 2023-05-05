use super::ast::*;

pub trait Visitor {
    fn visit_scene(self: &mut Self, node: &SceneNode);

    fn visit_object_compound(self: &mut Self, node: &ObjectCompoundNode);
    fn visit_object_simple(self: &mut Self, node: &ObjectSimpleNode);
    fn visit_object_transformed(self: &mut Self, node: &ObjectTransformedNode);

    fn visit_shape_aabox(self: &mut Self, node: &AABoxShapeNode);
    fn visit_shape_csg_elem(self: &mut Self, node: &CSGShapeElemNode);
    fn visit_shape_csg_intersection(self: &mut Self, node: &CSGShapeIntersectionNode);
    fn visit_shape_csg_substraction(self: &mut Self, node: &CSGShapeSubstractionNode);
    fn visit_shape_csg_union(self: &mut Self, node: &CSGShapeUnionNode);
    fn visit_shape_cylinder(self: &mut Self, node: &CylinderShapeNode);
    fn visit_shape_plane(self: &mut Self, node: &PlaneShapeNode);
    fn visit_shape_rectangle(self: &mut Self, node: &RectangleShapeNode);
    fn visit_shape_sphere(self: &mut Self, node: &SphereShapeNode);

    fn visit_material_lambertian(self: &mut Self, node: &LambertianMaterialNode);

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
