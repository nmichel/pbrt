mod ply_data_type;
mod ply_event_observer;
mod ply_prop_type;

use ply_data_type::PlyDataType;
use ply_event_observer::PlyEventObserver;
use ply_prop_type::ElementProps;

use std::fs::File;
use std::io::Read;

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

#[derive(Debug)]
enum State {
    Header,
    Format,
    Element,
    Property,
    Data,
}

#[derive(Debug)]
struct Builder {
    state: State,
    elements: Vec<ElementDesc>,
    current_element: Option<ElementDesc>,
    current_element_index: usize,
    current_element_count: usize,
}

type ElementParserFn = fn(&mut Builder, usize);

impl Builder {
    fn new() -> Builder {
        Builder {
            state: State::Header,
            elements: Vec::new(),
            current_element: None,
            current_element_index: 0,
            current_element_count: 0,
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

    fn load_data_line(&mut self, index: usize, segments: &Vec<&str>, observer: &mut dyn PlyEventObserver) {
        let element_desc: &ElementDesc = &self.elements[index];
        match &element_desc.name[..] {
            "vertex" => load_element_vertex(element_desc, segments, observer),
            "face" => load_element_face(element_desc, segments, observer),
            _ => {}
        }
    }
}

fn read_scalar_value(data_type: &PlyDataType, segments: &[&str]) -> PropertyValue {
    match data_type {
        PlyDataType::Int8 => PropertyValue::Int8(segments[0].parse::<i8>().unwrap()),
        PlyDataType::UInt8 => PropertyValue::UInt8(segments[0].parse::<u8>().unwrap()),
        PlyDataType::Int16 => PropertyValue::Int16(segments[0].parse::<i16>().unwrap()),
        PlyDataType::UInt16 => PropertyValue::UInt16(segments[0].parse::<u16>().unwrap()),
        PlyDataType::Int32 => PropertyValue::Int32(segments[0].parse::<i32>().unwrap()),
        PlyDataType::UInt32 => PropertyValue::UInt32(segments[0].parse::<u32>().unwrap()),
        PlyDataType::Float32 => PropertyValue::Float32(segments[0].parse::<f32>().unwrap()),
        PlyDataType::Float64 => PropertyValue::Float64(segments[0].parse::<f64>().unwrap()),
    }
}

fn read_list_value(count: usize, data_type: &PlyDataType, segments: &[&str]) -> PropertyValue {
    // Closure cannot be generic, hence we create a local helper function    //
    fn parse_list<T, F>(count: usize, segments: &[&str], parse: F) -> Vec<T>
    where
        F: Fn(&str) -> T,
    {
        segments[0..count].iter().map(|s| parse(s)).collect()
    }

    match data_type {
        PlyDataType::Int8 => PropertyValue::ListInt8(parse_list(count, segments, |s| s.parse::<i8>().unwrap())),
        PlyDataType::UInt8 => PropertyValue::ListUInt8(parse_list(count, segments, |s| s.parse::<u8>().unwrap())),
        PlyDataType::Int16 => PropertyValue::ListInt16(parse_list(count, segments, |s| s.parse::<i16>().unwrap())),
        PlyDataType::UInt16 => PropertyValue::ListUInt16(parse_list(count, segments, |s| s.parse::<u16>().unwrap())),
        PlyDataType::Int32 => PropertyValue::ListInt32(parse_list(count, segments, |s| s.parse::<i32>().unwrap())),
        PlyDataType::UInt32 => PropertyValue::ListUInt32(parse_list(count, segments, |s| s.parse::<u32>().unwrap())),
        PlyDataType::Float32 => PropertyValue::ListFloat32(parse_list(count, segments, |s| s.parse::<f32>().unwrap())),
        PlyDataType::Float64 => PropertyValue::ListFloat64(parse_list(count, segments, |s| s.parse::<f64>().unwrap())),
    }
}

fn read_value(segments: &[&str], data_type: &PropertyType) -> (usize, PropertyValue) {
    match data_type {
        PropertyType::Scalar(data_type) => (1, read_scalar_value(data_type, segments)),
        PropertyType::List(_count_type, _data_type) => {
            let count = segments[0].parse::<usize>().unwrap();
            (count + 1, read_list_value(count, _data_type, &segments[1..]))
        }
    }
}

fn dispatch_event(
    element_desc: &ElementDesc,
    segments: &Vec<&str>,
    observer: &mut dyn PlyEventObserver,
    handler: fn(&ElementProps, PropertyValue, &mut dyn PlyEventObserver),
) {
    let mut current_index = 0;
    for i in 0..element_desc.properties.len() {
        let segs = &segments[current_index..segments.len()];
        let property = &element_desc.properties[i];
        let (consumed, value) = read_value(segs, &property.data_type);
        let name = &property.name;
        handler(name, value, observer);
        current_index += consumed;
    }
}

fn load_element_vertex(element_desc: &ElementDesc, segments: &Vec<&str>, observer: &mut dyn PlyEventObserver) {
    fn handler(name: &ElementProps, value: PropertyValue, observer: &mut dyn PlyEventObserver) {
        observer.on_vertex_event(name, value);
    }

    dispatch_event(element_desc, segments, observer, handler);
}

fn load_element_face(element_desc: &ElementDesc, segments: &Vec<&str>, observer: &mut dyn PlyEventObserver) {
    fn handler(name: &ElementProps, value: PropertyValue, observer: &mut dyn PlyEventObserver) {
        observer.on_face_event(name, value);
    }

    dispatch_event(element_desc, segments, observer, handler);
}

fn read_ply_file(path: &str, observer: &mut dyn PlyEventObserver) -> Result<Builder, &'static str> {
    let mut f: File = File::open(path).unwrap();
    let mut buffer = String::new();
    f.read_to_string(&mut buffer).unwrap();

    read_ply_data(&buffer, observer)
}

pub fn read_ply_data(data: &str, observer: &mut dyn PlyEventObserver) -> Result<Builder, &'static str> {
    let mut state = Builder::new();

    let res = data
        .split('\n')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .filter(|x| !x.starts_with("comment"))
        .try_fold(&mut state, |state, line| read_line(line, state, observer));

    if res.is_err() {
        return Err(res.err().unwrap());
    }

    observer.on_data_complete();

    Ok(state)
}

fn read_line<'a>(line: &'a str, state: &'a mut Builder, observer: &mut dyn PlyEventObserver) -> Result<&'a mut Builder, &'static str> {
    let segments: Vec<&str> = line.split(" ").into_iter().map(|x| x.trim()).collect();
    parse_segments(&segments, state, observer)
}

fn parse_segments<'a>(segments: &Vec<&str>, state: &'a mut Builder, observer: &mut dyn PlyEventObserver) -> Result<&'a mut Builder, &'static str> {
    match state.state {
        State::Header => {
            match segments[..] {
                ["ply"] => {
                    state.state = State::Format;
                    Ok(state)
                }
                _ => Err("Header should start with 'ply'"),
            }
        }
        State::Format => {
            match segments[..] {
                ["format", "ascii", "1.0"] => {
                    state.state = State::Element;
                    Ok(state)
                }
                _ => Err("Format should be 'format ascii 1.0'"),
            }
        }
        State::Element => {
            match segments[..] {
                ["element", "vertex", count] => {
                    state.new_element("vertex", count.parse::<usize>().unwrap());
                    state.state = State::Property;
                    Ok(state)
                }
                ["element", "face", count] => {
                    state.new_element("face", count.parse::<usize>().unwrap());
                    state.state = State::Property;
                    Ok(state)
                }
                ["end_header"] => {
                    state.end_element();

                    observer.on_header_complete(&state.elements);

                    state.state = State::Data;
                    Ok(state)
                }
                _ => Err("Element should start with 'element vertex' or 'element face'"),
            }
        }
        State::Property => {
            if segments[0] != "property" {
                state.state = State::Element;
                return parse_segments(segments, state, observer);
            }

            match segments[..] {
                ["property", data_type, name] => {
                    state.state = State::Property;
                    state.current_element.as_mut().unwrap().properties.push(PropertyDesc {
                        name: name.parse::<ElementProps>().unwrap(),
                        data_type: PropertyType::Scalar(data_type.parse::<PlyDataType>().unwrap()),
                    });
                    Ok(state)
                }
                ["property", "list", count_type, data_type, name] => {
                    state.state = State::Property;
                    state.current_element.as_mut().unwrap().properties.push(PropertyDesc {
                        name: name.parse::<ElementProps>().unwrap(),
                        data_type: PropertyType::List(count_type.parse::<PlyDataType>().unwrap(), data_type.parse::<PlyDataType>().unwrap()),
                    });
                    Ok(state)
                }
                _ => {
                    state.state = State::Element;
                    return parse_segments(segments, state, observer);
                }
            }
        }
        State::Data => {
            state.current_element_count += 1;
            if state.current_element_count > state.elements[state.current_element_index].count {
                state.current_element_index += 1;
                state.current_element_count = 0;
            }
            state.load_data_line(state.current_element_index, segments, observer);
            Ok(state)
        }
    }
}

#[cfg(test)]
mod test {
    use super::ply_event_observer::PlyEventObserver;
    use super::ply_prop_type::*;
    use super::{read_ply_data, read_ply_file, ElementDesc, PropertyValue};

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
}
