use super::value::read_value;
use super::{ElementDesc, ElementProps, PlyEventObserver, PropertyValue, ValueDecoder};

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
pub(super) fn read_body(decoder: &mut dyn ValueDecoder, elements: &[ElementDesc], observer: &mut dyn PlyEventObserver) -> Result<(), String> {
    for element in elements {
        for _ in 0..element.count {
            load_element(element, decoder, observer)?;
        }
    }

    Ok(())
}
