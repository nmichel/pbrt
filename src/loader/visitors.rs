use super::ast::*;

pub trait Visitor {
    fn visit_scene(self: &mut Self, node: &SceneNode);
    fn visit_object_simple(self: &mut Self, node: &ObjectSimpleNode);
    fn visit_shape_sphere(self: &mut Self, node: &SphereShapeNode);
    fn visit_material_lambertian(self: &mut Self, node: &LambertianMaterialNode);
    fn visit_texture_color(self: &mut Self, node: &ColorTextureNode);
}

mod print;
mod scene_builder;

pub use self::print::PrintVisitor;
pub use self::scene_builder::SceneBuilderVisitor;
