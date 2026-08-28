use std::convert::TryFrom;

use super::ply_data_type::PlyDataType;
use super::ply_loader::{PropertyType, PropertyValue};

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
/// [`read_value`], shared by every implementation. Adding an encoding therefore means answering
/// eight questions about single values, never restating how a property or a list is laid out.
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

/// A list length read from the file is untrusted: a corrupt or misinterpreted count must not turn
/// into a huge allocation before the reads themselves fail on missing data. Preallocation is
/// therefore capped, and anything longer simply grows the vector as it goes. The value is far
/// above any real polygon.
const MAX_PREALLOCATED_LIST_LEN: usize = 1024;

/// Reads one property — scalar or list — from the decoder.
///
/// Takes `&mut dyn ValueDecoder` rather than a generic parameter, so that the encoding is resolved
/// once, where the decoder is built, instead of being threaded through every function below the
/// entry point. The price is a virtual call per primitive value, which is not free: parsing
/// `xyzrgb_dragon.ply` (3.6 M vertices, 7.2 M faces, ~33 M reads) takes 305 ms this way against
/// 261 ms when monomorphised, so +17 % on the parse alone.
///
/// That figure is quoted against a whole run, not against the parse: the same file spends 6.3 s in
/// load + BVH build, so the 44 ms is 0.7 % of loading it and a few tenths of a percent of a render.
/// Below the threshold where this project trades clarity for speed.
pub fn read_value(decoder: &mut dyn ValueDecoder, property_type: &PropertyType) -> Result<PropertyValue, String> {
    match property_type {
        PropertyType::Scalar(data_type) => read_scalar(decoder, *data_type),
        PropertyType::List(count_type, item_type) => {
            let count = read_count(decoder, *count_type)?;
            read_list(decoder, count, *item_type)
        }
    }
}

fn read_scalar(decoder: &mut dyn ValueDecoder, data_type: PlyDataType) -> Result<PropertyValue, String> {
    Ok(match data_type {
        PlyDataType::Int8 => PropertyValue::Int8(decoder.read_i8()?),
        PlyDataType::UInt8 => PropertyValue::UInt8(decoder.read_u8()?),
        PlyDataType::Int16 => PropertyValue::Int16(decoder.read_i16()?),
        PlyDataType::UInt16 => PropertyValue::UInt16(decoder.read_u16()?),
        PlyDataType::Int32 => PropertyValue::Int32(decoder.read_i32()?),
        PlyDataType::UInt32 => PropertyValue::UInt32(decoder.read_u32()?),
        PlyDataType::Float32 => PropertyValue::Float32(decoder.read_f32()?),
        PlyDataType::Float64 => PropertyValue::Float64(decoder.read_f64()?),
    })
}

/// Reads the length that prefixes a list property.
///
/// The header declares that length's own type (`property list uchar int vertex_indices` — a
/// `uchar` count, `int` items), and it must be honoured: in a binary body it decides whether one
/// or four bytes are consumed before the list itself, so getting it wrong desynchronises
/// everything that follows.
fn read_count(decoder: &mut dyn ValueDecoder, count_type: PlyDataType) -> Result<usize, String> {
    let count: i64 = match count_type {
        PlyDataType::Int8 => decoder.read_i8()? as i64,
        PlyDataType::UInt8 => decoder.read_u8()? as i64,
        PlyDataType::Int16 => decoder.read_i16()? as i64,
        PlyDataType::UInt16 => decoder.read_u16()? as i64,
        PlyDataType::Int32 => decoder.read_i32()? as i64,
        PlyDataType::UInt32 => decoder.read_u32()? as i64,
        PlyDataType::Float32 | PlyDataType::Float64 => return Err("A list count cannot be a floating point type".to_string()),
    };

    usize::try_from(count).map_err(|_| format!("Negative list count: {}", count))
}

fn read_list(decoder: &mut dyn ValueDecoder, count: usize, item_type: PlyDataType) -> Result<PropertyValue, String> {
    // Closures cannot be generic, hence a local helper function taking the read method as a value.
    fn collect<T, F>(decoder: &mut dyn ValueDecoder, count: usize, read: F) -> Result<Vec<T>, String>
    where
        F: Fn(&mut dyn ValueDecoder) -> Result<T, String>,
    {
        let mut items = Vec::with_capacity(count.min(MAX_PREALLOCATED_LIST_LEN));
        for _ in 0..count {
            items.push(read(decoder)?);
        }
        Ok(items)
    }

    Ok(match item_type {
        PlyDataType::Int8 => PropertyValue::ListInt8(collect(decoder, count, |d| d.read_i8())?),
        PlyDataType::UInt8 => PropertyValue::ListUInt8(collect(decoder, count, |d| d.read_u8())?),
        PlyDataType::Int16 => PropertyValue::ListInt16(collect(decoder, count, |d| d.read_i16())?),
        PlyDataType::UInt16 => PropertyValue::ListUInt16(collect(decoder, count, |d| d.read_u16())?),
        PlyDataType::Int32 => PropertyValue::ListInt32(collect(decoder, count, |d| d.read_i32())?),
        PlyDataType::UInt32 => PropertyValue::ListUInt32(collect(decoder, count, |d| d.read_u32())?),
        PlyDataType::Float32 => PropertyValue::ListFloat32(collect(decoder, count, |d| d.read_f32())?),
        PlyDataType::Float64 => PropertyValue::ListFloat64(collect(decoder, count, |d| d.read_f64())?),
    })
}
