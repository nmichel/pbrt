use super::format::PlyFormat;
use super::{ElementDesc, ElementProps, PlyDataType, PropertyDesc, PropertyType};

/// Everything the header declares.
#[derive(Debug)]
pub(super) struct PlyHeader {
    pub(super) format: PlyFormat,
    pub(super) elements: Vec<ElementDesc>,
    pub(super) body_offset: usize,
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

pub(super) fn read_header(data: &[u8]) -> Result<PlyHeader, String> {
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
