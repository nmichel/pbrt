use std::convert::TryFrom;
use std::fmt::Debug;

use super::ply::{read_ply_file, ElementDesc, ElementProps, PlyEventObserver, PropertyValue};
use crate::shapes::TriangleMesh;

/// Builds a `TriangleMesh` from a PLY file.
///
/// `reverse` flips the winding of every face. A PLY file carries no orientation
/// convention, so a mesh authored with clockwise faces comes out inside-out; reversing
/// the index triples is the fix, applied here rather than at intersection time so the
/// mesh is consistent for every later consumer.
pub fn load_ply_mesh(path: &str, reverse: bool) -> TriangleMesh {
    let mut observer = MeshBuilder {
        vertices: Vec::new(),
        faces: Vec::new(),
        reverse,
    };

    read_ply_file(path, &mut observer).unwrap();

    TriangleMesh::new(observer.vertices, observer.faces, None, None)
}

/// Accumulates the vertex coordinates and face indices announced by the PLY reader.
///
/// Vertex normals and texture coordinates are dropped: `TriangleMesh` does not use them
/// yet (see the "mesh normals are parsed and never used" entry in `IDEAS.md`).
struct MeshBuilder {
    vertices: Vec<f64>,
    faces: Vec<usize>,
    reverse: bool,
}

impl PlyEventObserver for MeshBuilder {
    fn on_header_complete(&mut self, _header: &Vec<ElementDesc>) {}

    fn on_vertex_start(&mut self) {}

    fn on_vertex_event(self: &mut Self, props: &ElementProps, value: PropertyValue) {
        match props {
            ElementProps::VertexX | ElementProps::VertexY | ElementProps::VertexZ => {
                self.vertices.push(f64::try_from(&value).unwrap());
            }
            _ => {}
        }
    }

    fn on_vertex_end(&mut self) {}

    fn on_face_start(&mut self) {}

    fn on_face_event(self: &mut Self, props: &ElementProps, value: PropertyValue) {
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
            ElementProps::FaceVertexIndices => {
                match value {
                    PropertyValue::ListUInt32(list) => {
                        push_to_faces(&list[..], &mut self.faces, self.reverse);
                    }
                    PropertyValue::ListInt32(list) => {
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
