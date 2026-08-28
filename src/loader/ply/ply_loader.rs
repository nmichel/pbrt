use super::ply_ascii_decoder::AsciiDecoder;
use super::ply_binary_decoder::{BinaryDecoder, Endianness};
use super::ply_data_type::PlyDataType;
use super::ply_event_observer::PlyEventObserver;
use super::ply_format::PlyFormat;
use super::ply_prop_type::ElementProps;
use super::ply_value_decoder::{read_value, ValueDecoder};

use std::convert::TryFrom;

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

/// Everything the header declares.
#[derive(Debug)]
struct PlyHeader {
    format: PlyFormat,
    elements: Vec<ElementDesc>,
    body_offset: usize,
}

#[derive(Debug)]
enum State {
    Header,
    Format,
    Element,
    Property,
}

#[derive(Debug)]
struct Builder {
    state: State,
    format: Option<PlyFormat>,
    elements: Vec<ElementDesc>,
    current_element: Option<ElementDesc>,
}

impl Builder {
    fn new() -> Builder {
        Builder {
            state: State::Header,
            format: None,
            elements: Vec::new(),
            current_element: None,
        }
    }

    fn new_element(&mut self, name: &str, count: usize) {
        match self.current_element {
            Some(ref element) => {
                self.elements.push(element.clone());
            }
            None => {}
        }
        self.current_element = Some(ElementDesc::new(name, count));
    }

    fn end_element(&mut self) {
        match self.current_element {
            Some(ref element) => {
                self.elements.push(element.clone());
            }
            None => {}
        }
        self.current_element = None;
    }
}

fn handle_vertex(name: &ElementProps, value: PropertyValue, observer: &mut dyn PlyEventObserver) {
    observer.on_vertex_event(name, value);
}

fn handle_face(name: &ElementProps, value: PropertyValue, observer: &mut dyn PlyEventObserver) {
    observer.on_face_event(name, value);
}

fn discard(_name: &ElementProps, _value: PropertyValue, _observer: &mut dyn PlyEventObserver) {}

/// Reads one occurrence of an element — every property the header declared for it, in order — and
/// hands each value to `handler`.
fn dispatch_event(
    element_desc: &ElementDesc,
    decoder: &mut dyn ValueDecoder,
    observer: &mut dyn PlyEventObserver,
    handler: fn(&ElementProps, PropertyValue, &mut dyn PlyEventObserver),
) -> Result<(), String> {
    for property in &element_desc.properties {
        let value = read_value(decoder, &property.data_type)?;
        handler(&property.name, value, observer);
    }

    Ok(())
}

fn load_element(element_desc: &ElementDesc, decoder: &mut dyn ValueDecoder, observer: &mut dyn PlyEventObserver) -> Result<(), String> {
    match &element_desc.name[..] {
        "vertex" => dispatch_event(element_desc, decoder, observer, handle_vertex),
        "face" => dispatch_event(element_desc, decoder, observer, handle_face),
        // An element we have no use for still has to be *read*. In a binary body nothing delimits
        // one value from the next, so its bytes can only be stepped over by decoding them; leaving
        // them in place would shift every element that follows.
        _ => dispatch_event(element_desc, decoder, observer, discard),
    }
}

/// Walks the data section: each element in the order the header declares it, repeated as many
/// times as the header announced.
///
/// The counts are the only structure the body has — in binary there are no line breaks to fall
/// back on, and in ASCII a value may not even sit on the line one would expect.
fn read_body(decoder: &mut dyn ValueDecoder, elements: &[ElementDesc], observer: &mut dyn PlyEventObserver) -> Result<(), String> {
    for element in elements {
        for _ in 0..element.count {
            load_element(element, decoder, observer)?;
        }
    }

    Ok(())
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

fn read_header(data: &[u8]) -> Result<PlyHeader, String> {
    let mut builder = Builder::new();
    let mut offset = 0;

    loop {
        let (line, next_offset) = next_header_line(data, offset)?;
        offset = next_offset;

        let line = line.trim();
        if line.is_empty() || line.starts_with("comment") || line.starts_with("obj_info") {
            continue;
        }

        let segments: Vec<&str> = line.split_whitespace().collect();
        if parse_header_line(&segments, &mut builder)? {
            break;
        }
    }

    let format = builder.format.ok_or_else(|| "Header has no 'format' line".to_string())?;

    Ok(PlyHeader {
        format: format,
        elements: builder.elements,
        body_offset: offset,
    })
}

/// Returns the next header line and the offset just past its terminating newline.
///
/// The header is read from the raw bytes rather than from a `str`: in the binary encodings the
/// bytes that follow `end_header` are not text, so the file cannot be validated as UTF-8 as a
/// whole. Each header line is validated on its own instead.
fn next_header_line(data: &[u8], offset: usize) -> Result<(&str, usize), String> {
    if offset >= data.len() {
        return Err("Unexpected end of header: 'end_header' was never reached".to_string());
    }

    let (line_bytes, next_offset) = match data[offset..].iter().position(|byte| *byte == b'\n') {
        Some(index) => (&data[offset..offset + index], offset + index + 1),
        None => (&data[offset..], data.len()),
    };

    let line = std::str::from_utf8(line_bytes).map_err(|err| format!("Header line at byte {} is not valid UTF-8: {}", offset, err))?;

    Ok((line, next_offset))
}

/// Feeds one header line to the state machine. Returns `true` once `end_header` closes it.
fn parse_header_line(segments: &[&str], builder: &mut Builder) -> Result<bool, String> {
    match builder.state {
        State::Header => {
            match segments[..] {
                ["ply"] => {
                    builder.state = State::Format;
                    Ok(false)
                }
                _ => Err("Header should start with 'ply'".to_string()),
            }
        }
        State::Format => {
            match segments[..] {
                ["format", encoding, "1.0"] => {
                    builder.format = Some(encoding.parse::<PlyFormat>()?);
                    builder.state = State::Element;
                    Ok(false)
                }
                _ => Err("Format should be 'format <ascii|binary_little_endian|binary_big_endian> 1.0'".to_string()),
            }
        }
        State::Element => {
            match segments[..] {
                // Any element name is accepted, not just 'vertex' and 'face': an element we ignore
                // still has to be declared for the ones after it to be found at the right place.
                ["element", name, count] => {
                    let count = count
                        .parse::<usize>()
                        .map_err(|err| format!("Invalid count '{}' for element '{}': {}", count, name, err))?;
                    builder.new_element(name, count);
                    builder.state = State::Property;
                    Ok(false)
                }
                ["end_header"] => {
                    builder.end_element();
                    Ok(true)
                }
                _ => Err(format!("Expected an 'element' or 'end_header' line, found '{}'", segments.join(" "))),
            }
        }
        State::Property => {
            if segments[0] != "property" {
                builder.state = State::Element;
                return parse_header_line(segments, builder);
            }

            let property = match segments[..] {
                ["property", data_type, name] => {
                    PropertyDesc {
                        name: name.parse::<ElementProps>()?,
                        data_type: PropertyType::Scalar(data_type.parse::<PlyDataType>()?),
                    }
                }
                ["property", "list", count_type, data_type, name] => {
                    PropertyDesc {
                        name: name.parse::<ElementProps>()?,
                        data_type: PropertyType::List(count_type.parse::<PlyDataType>()?, data_type.parse::<PlyDataType>()?),
                    }
                }
                _ => return Err(format!("Malformed property line: '{}'", segments.join(" "))),
            };

            builder
                .current_element
                .as_mut()
                .ok_or_else(|| "A 'property' line must follow an 'element' line".to_string())?
                .properties
                .push(property);

            Ok(false)
        }
    }
}

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
