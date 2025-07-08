mod ply_data_type;

use std::fs::File;
use std::io::Read;

use ply_data_type::PlyDataType;

#[derive(Debug, PartialEq, Eq, Clone)]
enum PropertyDesc {
    SimplePropertyDesc { name: String, data_type: PlyDataType },
    ListPropertyDesc { name: String, data_type: PlyDataType },
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct ElementDesc {
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
struct ArraySizes {
    vertex: usize,
    normal: usize,
    color: usize,
    uv: usize,
    indices: usize,
}

impl ArraySizes {
    fn new() -> Self {
        ArraySizes {
            vertex: 0,
            normal: 0,
            color: 0,
            uv: 0,
            indices: 0,
        }
    }
}
#[derive(Debug)]
struct Builder {
    state: State,
    elements: Vec<ElementDesc>,
    current_element: Option<ElementDesc>,
    sizes: ArraySizes,
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
            sizes: ArraySizes::new(),
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

    fn prepare_arrays(&mut self) -> () {
        for element in &self.elements {
            match &element.name[..] {
                "vertex" => process_element_vertex(&mut self.sizes, &element),
                _ => {}
            }
        }
    }

    fn load_data_line(&mut self, index: usize, segments: &Vec<&str>) {
        let element_desc: &ElementDesc = &self.elements[index];
        match &element_desc.name[..] {
            "vertex" => load_element_vertex(element_desc, segments),
            "face" => load_element_face(element_desc, segments),
            _ => {}
        }
    }
}

fn load_element_vertex(element_desc: &ElementDesc, segments: &Vec<&str>) {
    for i in 0..element_desc.properties.len() {
        let property = &element_desc.properties[i];
        let str = segments.get(i).expect("Missing data for property");
        match property {
            PropertyDesc::SimplePropertyDesc { name, .. } => {
                match &name[..] {
                    "x" | "y" | "z" => println!("Vertex coordinate {}: {}", name, str),
                    "nx" | "ny" | "nz" => println!("Vertex normal: {}: {}", name, str),
                    "red" | "tgreen" | "blue" => println!("Vertex color: {}: {}", name, str),
                    "u" | "v" | "s" | "t" => println!("Vertex uv: {}: {}", name, str),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn load_element_face(element_desc: &ElementDesc, segments: &Vec<&str>) {
    for i in 0..element_desc.properties.len() {
        let property = &element_desc.properties[i];
        match property {
            PropertyDesc::ListPropertyDesc { name, .. } => {
                let count = segments.get(0).expect("Missing vertex count for face");
                let indices = segments[1..].iter().map(|x| x.parse::<usize>().unwrap()).collect::<Vec<usize>>();
                match &name[..] {
                    "vertex_index" | "vertex_indices" => println!("Face vertex indices {} | {:#?}", count, indices),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn process_element_vertex(sizes: &mut ArraySizes, element_desc: &ElementDesc) {
    for property in &element_desc.properties {
        match property {
            PropertyDesc::SimplePropertyDesc { name, .. } => {
                match &name[..] {
                    "x" | "y" | "z" => {
                        sizes.vertex += element_desc.count;
                    }
                    "nx" | "ny" | "nz" => {
                        sizes.normal += element_desc.count;
                    }
                    "red" | "green" | "blue" => {
                        sizes.color += element_desc.count;
                    }
                    "u" | "v" => {
                        sizes.uv += element_desc.count;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

pub fn read_ply_file(path: &str) -> Result<Builder, &'static str> {
    let mut f: File = File::open(path).unwrap();
    let mut buffer = String::new();
    f.read_to_string(&mut buffer).unwrap();

    read_ply_data(&buffer)
}

pub fn read_ply_data(data: &str) -> Result<Builder, &'static str> {
    let mut state = Builder::new();

    let res = data
        .split('\n')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .filter(|x| !x.starts_with("comment"))
        .try_fold(&mut state, |state, line| read_line(line, state));

    if res.is_err() {
        return Err(res.err().unwrap());
    }

    Ok(state)
}

fn read_line<'a>(line: &'a str, state: &'a mut Builder) -> Result<&'a mut Builder, &'static str> {
    let segments: Vec<&str> = line.split(" ").into_iter().map(|x| x.trim()).collect();
    parse_segments(&segments, state)
}

fn parse_segments<'a>(segments: &Vec<&str>, state: &'a mut Builder) -> Result<&'a mut Builder, &'static str> {
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
                    state.prepare_arrays();

                    state.state = State::Data;
                    Ok(state)
                }
                _ => Err("Element should start with 'element vertex' or 'element face'"),
            }
        }
        State::Property => {
            if segments[0] != "property" {
                state.state = State::Element;
                return parse_segments(segments, state);
            }

            match segments[..] {
                ["property", data_type, name] => {
                    state.state = State::Property;
                    state.current_element.as_mut().unwrap().properties.push(PropertyDesc::SimplePropertyDesc {
                        name: name.to_string(),
                        data_type: data_type.parse::<PlyDataType>().unwrap(),
                    });
                    Ok(state)
                }
                ["property", "list", _count_type, data_type, name] => {
                    state.state = State::Property;
                    state.current_element.as_mut().unwrap().properties.push(PropertyDesc::ListPropertyDesc {
                        name: name.to_string(),
                        data_type: data_type.parse::<PlyDataType>().unwrap(),
                    });
                    Ok(state)
                }
                _ => {
                    state.state = State::Element;
                    return parse_segments(segments, state);
                }
            }
        }
        State::Data => {
            state.current_element_count += 1;
            if state.current_element_count > state.elements[state.current_element_index].count {
                state.current_element_index += 1;
                state.current_element_count = 0;
            }
            state.load_data_line(state.current_element_index, segments);

            {
                state.state = State::Data;
                Ok(state)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::read_ply_data;

    #[test]
    fn test_ply_reader_no_ply() {
        let input = "
        not ply
        ";

        assert!(read_ply_data(input).is_err());
    }

    #[test]
    fn test_ply_reader_ply_not_alone() {
        let input = "
        ply suffix
        ";

        assert!(read_ply_data(input).is_err());
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

        assert!(read_ply_data(input).is_ok());
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

        assert!(read_ply_data(input).is_ok());
    }
}
