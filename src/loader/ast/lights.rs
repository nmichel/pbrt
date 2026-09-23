use crate::loader::visitors::Visitor;
use crate::spectrum::Spectrum;

use super::{Node, TransformNode};

/// A light source the description declares on its own, rather than one a surface carries.
///
/// These are the lights with **no geometry**: nothing in the scene can be hit and found to be
/// one of them. An emissive surface is not declared here — it is an `object` wearing a
/// `diffuse_light` material, and the light that samples it is derived from that object. Two
/// syntaxes for one concept is what this split avoids.
pub trait LightNode: Node {}

pub struct PointLightNode {
    pub intensity: Spectrum,

    /// Where the light sits, said the way an object says it. The point itself is the origin of
    /// the frame this places, so a light with no `translate` sits at the world origin.
    pub transform: Box<TransformNode>,
}

impl LightNode for PointLightNode {}

impl Node for PointLightNode {
    /// The placement is visited first, as it is for a `csg elem`: the visitor finds it on its
    /// transformation stack when told about the light.
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        self.transform.visit(visitor);
        visitor.visit_light_point(self);
    }
}

impl PointLightNode {
    pub fn new(intensity: Spectrum, transform: Box<TransformNode>) -> Self {
        Self { intensity, transform }
    }
}

pub struct UniformInfiniteLightNode {
    pub radiance: Spectrum,
}

impl LightNode for UniformInfiniteLightNode {}

impl Node for UniformInfiniteLightNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        visitor.visit_light_uniform_infinite(self);
    }
}

impl UniformInfiniteLightNode {
    pub fn new(radiance: Spectrum) -> Self {
        Self { radiance }
    }
}

/// A sky: two colours the light blends between, bottom to top.
pub struct BackgroundInfiniteLightNode {
    pub bottom: Spectrum,
    pub top: Spectrum,
}

impl LightNode for BackgroundInfiniteLightNode {}

impl Node for BackgroundInfiniteLightNode {
    fn visit(self: &Self, visitor: &mut dyn Visitor) {
        visitor.visit_light_background_infinite(self);
    }
}

impl BackgroundInfiniteLightNode {
    pub fn new(bottom: Spectrum, top: Spectrum) -> Self {
        Self { bottom, top }
    }
}
