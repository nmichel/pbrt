use super::ply_prop_type::ElementProps;
use super::{ElementDesc, PropertyValue};

/// Trait for observing events during the parsing of PLY files.
pub trait PlyEventObserver {
    fn on_header_complete(&mut self, header: &Vec<ElementDesc>);

    fn on_vertex_event(&mut self, props: &ElementProps, value: PropertyValue);

    fn on_face_event(&mut self, props: &ElementProps, value: PropertyValue);

    fn on_data_complete(&mut self);
}
