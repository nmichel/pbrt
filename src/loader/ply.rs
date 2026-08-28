use std::convert::TryFrom;

use self::ascii_decoder::AsciiDecoder;
use self::binary_decoder::{BinaryDecoder, Endianness};
use self::body::read_body;
use self::format::PlyFormat;
use self::header::read_header;

#[derive(Debug, Clone)]
pub enum PropertyType {
    Scalar(PlyDataType),
    List(PlyDataType, PlyDataType),
}

#[derive(Debug)]
pub enum PropertyValue {
    Int8(i8),
    UInt8(u8),
    Int16(i16),
    UInt16(u16),
    Int32(i32),
    UInt32(u32),
    Float32(f32),
    Float64(f64),
    ListInt8(Vec<i8>),
    ListUInt8(Vec<u8>),
    ListInt16(Vec<i16>),
    ListUInt16(Vec<u16>),
    ListInt32(Vec<i32>),
    ListUInt32(Vec<u32>),
    ListFloat32(Vec<f32>),
    ListFloat64(Vec<f64>),
}

impl TryFrom<&PropertyValue> for f64 {
    type Error = &'static str;

    fn try_from(value: &PropertyValue) -> Result<Self, Self::Error> {
        match value {
            PropertyValue::Float32(f) => Ok(*f as f64),
            PropertyValue::Float64(f) => Ok(*f),
            _ => Err("Cannot convert non-float value to f64"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PropertyDesc {
    name: ElementProps,
    data_type: PropertyType,
}

#[derive(Debug, Clone)]
pub struct ElementDesc {
    name: String,
    count: usize,
    properties: Vec<PropertyDesc>,
}

impl ElementDesc {
    fn new(name: &str, count: usize) -> ElementDesc {
        ElementDesc {
            name: name.to_string(),
            count: count,
            properties: Vec::new(),
        }
    }
}

/// Trait for observing events during the parsing of PLY files.
pub trait PlyEventObserver {
    fn on_header_complete(&mut self, header: &Vec<ElementDesc>) {}

    fn on_vertex_start(&mut self) {}
    fn on_vertex_event(&mut self, props: &ElementProps, value: PropertyValue) {}
    fn on_vertex_end(&mut self) {}

    fn on_face_start(&mut self) {}
    fn on_face_event(&mut self, props: &ElementProps, value: PropertyValue) {}
    fn on_face_end(&mut self) {}

    fn on_data_complete(&mut self) {}
}

/// Turns the next block of data of a PLY body into a primitive value.
///
/// This is the single seam between *what* a PLY property is — a type read from the header — and
/// *how* its bytes are spelled on disk. A PLY body holds the very same sequence of values in all
/// three encodings of the format; only the spelling of one value changes:
///
/// - in `ascii`, a block is a whitespace-separated token, and the conversion is a decimal parse;
/// - in the two binary variants, a block is a fixed number of bytes, and the conversion is a
///   reinterpretation under a declared byte order.
///
/// The trait is deliberately placed at the *primitive* level, below the notion of property. It
/// knows nothing of `PropertyValue`, of lists, or of elements: everything above lives once in
/// `value::read_value`, shared by every implementation. Adding an encoding therefore means
/// answering eight questions about single values, never restating how a property or a list is
/// laid out.
///
/// Implementations are stateful — each call consumes the block it reads and advances to the next.
pub trait ValueDecoder {
    fn read_i8(&mut self) -> Result<i8, String>;
    fn read_u8(&mut self) -> Result<u8, String>;
    fn read_i16(&mut self) -> Result<i16, String>;
    fn read_u16(&mut self) -> Result<u16, String>;
    fn read_i32(&mut self) -> Result<i32, String>;
    fn read_u32(&mut self) -> Result<u32, String>;
    fn read_f32(&mut self) -> Result<f32, String>;
    fn read_f64(&mut self) -> Result<f64, String>;
}

pub fn read_ply_file(path: &str, observer: &mut dyn PlyEventObserver) -> Result<(), String> {
    let data = std::fs::read(path).map_err(|err| format!("Cannot read '{}': {}", path, err))?;

    read_ply_bytes(&data, observer)
}

pub fn read_ply_data(data: &str, observer: &mut dyn PlyEventObserver) -> Result<(), String> {
    read_ply_bytes(data.as_bytes(), observer)
}

/// Reads a PLY file of any of the three encodings the format defines.
///
/// The header is ASCII in all of them and the body is not, which is why the whole file is handled
/// as bytes: the split between the two is `end_header`, and only past that point does the
/// announced encoding decide how a value is spelled. Beyond that choice of decoder the three
/// encodings share everything — the same elements, the same properties, and therefore the same
/// sequence of observer events.
pub fn read_ply_bytes(data: &[u8], observer: &mut dyn PlyEventObserver) -> Result<(), String> {
    let header = read_header(data)?;

    observer.on_header_complete(&header.elements);

    let body = &data[header.body_offset..];
    let mut decoder: Box<dyn ValueDecoder + '_> = match header.format {
        PlyFormat::Ascii => {
            let text = std::str::from_utf8(body).map_err(|err| format!("Ascii data section is not valid UTF-8: {}", err))?;
            Box::new(AsciiDecoder::new(text))
        }
        PlyFormat::BinaryLittleEndian => Box::new(BinaryDecoder::new(body, Endianness::Little)),
        PlyFormat::BinaryBigEndian => Box::new(BinaryDecoder::new(body, Endianness::Big)),
    };

    read_body(decoder.as_mut(), &header.elements, observer)?;

    observer.on_data_complete();

    Ok(())
}

mod ascii_decoder;
mod binary_decoder;
mod body;
mod data_type;
mod element_props;
mod format;
mod header;
mod value;

pub use self::data_type::PlyDataType;
pub use self::element_props::ElementProps;

#[cfg(test)]
mod test {
    use super::{read_ply_bytes, read_ply_data, read_ply_file, ElementDesc, ElementProps, PlyEventObserver, PropertyValue};

    use std::convert::TryFrom;

    struct MockEventObserver {}

    impl MockEventObserver {
        fn new() -> Self {
            MockEventObserver {}
        }
    }

    impl PlyEventObserver for MockEventObserver {
        fn on_header_complete(self: &mut Self, header: &Vec<ElementDesc>) {
            for element in header {
                println!("Element: {}", element.name);
                for property in &element.properties {
                    println!("  Property: {:?}", property);
                }
            }
        }

        fn on_vertex_event(self: &mut Self, props: &ElementProps, value: PropertyValue) {
            println!("Vertex Event for Property: {:?} with Value: {:?}", props, value);
        }

        fn on_face_event(self: &mut Self, props: &ElementProps, value: PropertyValue) {
            println!("Face Event for Property: {:?} with Value: {:?}", props, value);
        }

        fn on_data_complete(self: &mut Self) {
            println!("Data Complete");
        }
    }

    #[test]
    fn test_ply_reader_no_ply() {
        let input = "
        not ply
        ";

        assert!(read_ply_data(input, &mut MockEventObserver::new()).is_err());
    }

    #[test]
    fn test_ply_reader_ply_not_alone() {
        let input = "
        ply suffix
        ";

        assert!(read_ply_data(input, &mut MockEventObserver::new()).is_err());
    }

    #[test]
    fn test_ply_reader() {
        let input = "
        ply
        format ascii 1.0
        comment made by Greg Turk
        comment this file is a cube
        element vertex 8
        property float x
        property float y
        property float z
        element face 6
        property list uchar int vertex_index
        end_header
        0 0 0
        0 0 1
        0 1 1
        0 1 0
        1 0 0
        1 0 1
        1 1 1
        1 1 0
        4 0 1 2 3
        4 7 6 5 4
        4 0 4 5 1
        4 1 5 6 2
        4 2 6 7 3
        4 3 7 4 0
        ";

        assert!(read_ply_data(input, &mut MockEventObserver::new()).is_ok());
    }

    #[test]
    fn test_ply_reader_many_vert_props() {
        let input = "
        ply
        format ascii 1.0
        comment Created in Blender version 4.0.2
        element vertex 14
        property float x
        property float y
        property float z
        property float nx
        property float ny
        property float nz
        property float s
        property float t
        element face 6
        property list uchar uint vertex_indices
        end_header
        1 1 1 0.5773503 0.5773503 0.5773503 0.625 0.5
        -1 1 1 -0.5773503 0.5773503 0.5773503 0.875 0.5
        -1 -1 1 -0.5773503 -0.5773503 0.5773503 0.875 0.75
        1 -1 1 0.5773503 -0.5773503 0.5773503 0.625 0.75
        1 -1 -1 0.5773503 -0.5773503 -0.5773503 0.375 0.75
        -1 -1 1 -0.5773503 -0.5773503 0.5773503 0.625 1
        -1 -1 -1 -0.5773503 -0.5773503 -0.5773503 0.375 1
        -1 -1 -1 -0.5773503 -0.5773503 -0.5773503 0.375 0
        -1 -1 1 -0.5773503 -0.5773503 0.5773503 0.625 0
        -1 1 1 -0.5773503 0.5773503 0.5773503 0.625 0.25
        -1 1 -1 -0.5773503 0.5773503 -0.5773503 0.375 0.25
        -1 1 -1 -0.5773503 0.5773503 -0.5773503 0.125 0.5
        1 1 -1 0.5773503 0.5773503 -0.5773503 0.375 0.5
        -1 -1 -1 -0.5773503 -0.5773503 -0.5773503 0.125 0.75
        4 0 1 2 3
        4 4 3 5 6
        4 7 8 9 10
        4 11 12 4 13
        4 12 0 3 4
        4 10 9 0 12
        ";

        assert!(read_ply_data(input, &mut MockEventObserver::new()).is_ok());
    }

    #[test]
    fn test_read_bunny() {
        let filename = "./test_files/bun_zipper.ply";
        let mut observer = MockEventObserver::new();
        assert!(read_ply_file(&filename, &mut observer).is_ok());
    }

    /// Records every value it is handed, so that two encodings can be compared value by value.
    ///
    /// The printing mock above cannot catch a byte-order mistake: a big-endian file read as
    /// little-endian produces the same *number* of events, only wrong ones.
    #[derive(Default, PartialEq, Debug)]
    struct RecordingObserver {
        vertices: Vec<f64>,
        faces: Vec<i32>,
    }

    impl PlyEventObserver for RecordingObserver {
        fn on_vertex_event(self: &mut Self, _props: &ElementProps, value: PropertyValue) {
            self.vertices.push(f64::try_from(&value).unwrap());
        }

        fn on_face_event(self: &mut Self, _props: &ElementProps, value: PropertyValue) {
            match value {
                PropertyValue::ListInt32(list) => self.faces.extend(list),
                other => panic!("Unexpected face value: {:?}", other),
            }
        }
    }

    const CUBE_VERTICES: [[f32; 3]; 8] = [
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 1.0, 1.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 0.0],
    ];

    const CUBE_FACES: [[i32; 4]; 6] = [[0, 1, 2, 3], [7, 6, 5, 4], [0, 4, 5, 1], [1, 5, 6, 2], [2, 6, 7, 3], [3, 7, 4, 0]];

    fn cube_header(encoding: &str) -> String {
        format!(
            "ply\nformat {} 1.0\nelement vertex 8\nproperty float x\nproperty float y\nproperty float z\nelement face 6\nproperty list uchar int \
             vertex_index\nend_header\n",
            encoding
        )
    }

    fn cube_ascii() -> Vec<u8> {
        let mut text = cube_header("ascii");

        for vertex in CUBE_VERTICES {
            text.push_str(&format!("{} {} {}\n", vertex[0], vertex[1], vertex[2]));
        }
        for face in CUBE_FACES {
            text.push_str(&format!("4 {} {} {} {}\n", face[0], face[1], face[2], face[3]));
        }

        text.into_bytes()
    }

    fn cube_binary(encoding: &str, big_endian: bool) -> Vec<u8> {
        fn push_f32(data: &mut Vec<u8>, value: f32, big_endian: bool) {
            let bytes = if big_endian { value.to_be_bytes() } else { value.to_le_bytes() };
            data.extend_from_slice(&bytes);
        }

        fn push_i32(data: &mut Vec<u8>, value: i32, big_endian: bool) {
            let bytes = if big_endian { value.to_be_bytes() } else { value.to_le_bytes() };
            data.extend_from_slice(&bytes);
        }

        let mut data = cube_header(encoding).into_bytes();

        for vertex in CUBE_VERTICES {
            for coordinate in vertex {
                push_f32(&mut data, coordinate, big_endian);
            }
        }
        for face in CUBE_FACES {
            data.push(CUBE_FACES[0].len() as u8); // the 'uchar' list count
            for index in face {
                push_i32(&mut data, index, big_endian);
            }
        }

        data
    }

    fn record(data: &[u8]) -> RecordingObserver {
        let mut observer = RecordingObserver::default();
        read_ply_bytes(data, &mut observer).unwrap();
        observer
    }

    /// The point of the whole exercise: the encoding changes how a value is spelled, never which
    /// values a file holds. The three readings must be indistinguishable.
    #[test]
    fn test_binary_and_ascii_agree() {
        let ascii = record(&cube_ascii());
        let big_endian = record(&cube_binary("binary_big_endian", true));
        let little_endian = record(&cube_binary("binary_little_endian", false));

        assert_eq!(ascii.vertices.len(), 24);
        assert_eq!(ascii.faces.len(), 24);

        assert_eq!(ascii, big_endian);
        assert_eq!(ascii, little_endian);
    }

    /// Guards against a decoder that ignores the declared byte order: read under the wrong one,
    /// the very same bytes must yield something else. Without this, a reader hard-wired to the
    /// host order would pass the test above.
    #[test]
    fn test_byte_order_is_honoured() {
        let mislabelled = cube_binary("binary_little_endian", true);

        assert_ne!(record(&mislabelled).vertices, record(&cube_ascii()).vertices);
    }

    /// An element the loader has no use for must still be consumed. Its bytes have no delimiter,
    /// so getting this wrong shifts every element declared after it.
    #[test]
    fn test_unknown_element_is_consumed() {
        let mut data = "ply\nformat binary_big_endian 1.0\nelement vertex 1\nproperty float x\nproperty float y\nproperty float z\nelement camera \
                        2\nproperty double view_x\nproperty list uchar int keys\nelement face 1\nproperty list uchar int vertex_index\nend_header\n"
            .to_string()
            .into_bytes();

        data.extend_from_slice(&1.0f32.to_be_bytes());
        data.extend_from_slice(&2.0f32.to_be_bytes());
        data.extend_from_slice(&3.0f32.to_be_bytes());
        for _ in 0..2 {
            data.extend_from_slice(&9.0f64.to_be_bytes());
            data.push(2);
            data.extend_from_slice(&7i32.to_be_bytes());
            data.extend_from_slice(&8i32.to_be_bytes());
        }
        data.push(3);
        for index in [0i32, 1, 2] {
            data.extend_from_slice(&index.to_be_bytes());
        }

        let observer = record(&data);

        assert_eq!(observer.vertices, vec![1.0, 2.0, 3.0]);
        assert_eq!(observer.faces, vec![0, 1, 2]);
    }

    #[test]
    fn test_unknown_format_is_an_error() {
        let input = "ply\nformat binary 1.0\nend_header\n";

        assert!(read_ply_data(input, &mut MockEventObserver::new()).is_err());
    }

    #[test]
    fn test_truncated_binary_body_is_an_error() {
        let mut data = cube_binary("binary_big_endian", true);
        data.truncate(data.len() - 1);

        assert!(read_ply_bytes(&data, &mut RecordingObserver::default()).is_err());
    }

    #[test]
    fn test_header_without_end_header_is_an_error() {
        let input = "ply\nformat ascii 1.0\nelement vertex 1\nproperty float x\n";

        assert!(read_ply_data(input, &mut MockEventObserver::new()).is_err());
    }
}
