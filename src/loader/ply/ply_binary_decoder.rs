use std::convert::TryInto;

use super::ply_value_decoder::ValueDecoder;

/// The order in which a multi-byte scalar is laid out in a binary PLY body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    Little,
    Big,
}

/// Reads the body of a `binary_little_endian` or `binary_big_endian` PLY file.
///
/// Reference: Greg Turk, *The PLY Polygon File Format*, section on binary files — the data section
/// is the concatenation, with no padding, separator or alignment, of every property of every
/// element, in the order the header declares them.
///
/// # Framing
///
/// A block of data here is a fixed run of bytes whose length is that of the declared type: 1 byte
/// for `char`/`uchar`, 2 for `short`/`ushort`, 4 for `int`/`uint`/`float`, 8 for `double`. Since
/// nothing separates two values, the only thing keeping the reader in step with the file is the
/// header: read one property with the wrong width and everything after it is misaligned. This is
/// why the count type of a list property has to be honoured — a `uchar` count consumes one byte
/// where an `int` count consumes four.
///
/// # Byte order
///
/// The two variants differ only in how the bytes of one scalar are ordered. Taking `1.0f32`, whose
/// IEEE-754 encoding is `0x3F800000`:
///
/// - big-endian    — most significant byte first: `3F 80 00 00`
/// - little-endian — least significant byte first: `00 00 80 3F`
///
/// The order is announced by the `format` header line and by nothing else: there is no marker in
/// the data, so reading a file under the wrong order is silent and yields plausible-looking
/// garbage rather than an error.
///
/// The conversion is therefore always explicit — `from_be_bytes` / `from_le_bytes` — and never a
/// reinterpretation of host memory. x86 and ARM are little-endian, so a big-endian file such as
/// `test_files/xyzrgb_dragon.ply` genuinely needs its bytes swapped; assuming the host order would
/// happen to work for one variant and corrupt the other.
///
/// The endianness is a field tested on each read rather than a type parameter: the branch is
/// perfectly predicted, and the alternative buys nothing but a second monomorphisation.
pub struct BinaryDecoder<'a> {
    data: &'a [u8],
    offset: usize,
    endianness: Endianness,
}

impl<'a> BinaryDecoder<'a> {
    pub fn new(data: &'a [u8], endianness: Endianness) -> BinaryDecoder<'a> {
        BinaryDecoder { data, offset: 0, endianness }
    }

    /// Consumes the next `N` bytes of the body.
    ///
    /// The offset is reported on failure: in a binary file there is no line number to point at,
    /// and the byte at which the reader ran out is the only handle on where the header and the
    /// data stopped agreeing.
    fn next_bytes<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let end = self.offset + N;
        if end > self.data.len() {
            return Err(format!(
                "Unexpected end of data at byte {}: {} bytes needed, {} available",
                self.offset,
                N,
                self.data.len() - self.offset
            ));
        }

        // The slice is exactly N bytes long by construction, so the conversion cannot fail.
        let bytes: [u8; N] = self.data[self.offset..end].try_into().unwrap();
        self.offset = end;

        Ok(bytes)
    }
}

impl<'a> ValueDecoder for BinaryDecoder<'a> {
    fn read_i8(&mut self) -> Result<i8, String> {
        // A single byte has no byte order.
        Ok(i8::from_be_bytes(self.next_bytes::<1>()?))
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        Ok(u8::from_be_bytes(self.next_bytes::<1>()?))
    }

    fn read_i16(&mut self) -> Result<i16, String> {
        let bytes = self.next_bytes::<2>()?;
        Ok(match self.endianness {
            Endianness::Big => i16::from_be_bytes(bytes),
            Endianness::Little => i16::from_le_bytes(bytes),
        })
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let bytes = self.next_bytes::<2>()?;
        Ok(match self.endianness {
            Endianness::Big => u16::from_be_bytes(bytes),
            Endianness::Little => u16::from_le_bytes(bytes),
        })
    }

    fn read_i32(&mut self) -> Result<i32, String> {
        let bytes = self.next_bytes::<4>()?;
        Ok(match self.endianness {
            Endianness::Big => i32::from_be_bytes(bytes),
            Endianness::Little => i32::from_le_bytes(bytes),
        })
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let bytes = self.next_bytes::<4>()?;
        Ok(match self.endianness {
            Endianness::Big => u32::from_be_bytes(bytes),
            Endianness::Little => u32::from_le_bytes(bytes),
        })
    }

    fn read_f32(&mut self) -> Result<f32, String> {
        let bytes = self.next_bytes::<4>()?;
        Ok(match self.endianness {
            Endianness::Big => f32::from_be_bytes(bytes),
            Endianness::Little => f32::from_le_bytes(bytes),
        })
    }

    fn read_f64(&mut self) -> Result<f64, String> {
        let bytes = self.next_bytes::<8>()?;
        Ok(match self.endianness {
            Endianness::Big => f64::from_be_bytes(bytes),
            Endianness::Little => f64::from_le_bytes(bytes),
        })
    }
}

#[cfg(test)]
mod test {
    use super::{BinaryDecoder, Endianness, ValueDecoder};

    /// 1.0f32 is 0x3F800000, which makes the byte order plainly visible in the fixture.
    #[test]
    fn test_reads_float_big_endian() {
        let data = [0x3F, 0x80, 0x00, 0x00];
        let mut decoder = BinaryDecoder::new(&data, Endianness::Big);

        assert_eq!(decoder.read_f32().unwrap(), 1.0);
    }

    #[test]
    fn test_reads_float_little_endian() {
        let data = [0x00, 0x00, 0x80, 0x3F];
        let mut decoder = BinaryDecoder::new(&data, Endianness::Little);

        assert_eq!(decoder.read_f32().unwrap(), 1.0);
    }

    /// The same bytes under the two orders must disagree — otherwise the endianness field is being
    /// ignored and every big-endian file would silently decode as little-endian.
    #[test]
    fn test_byte_order_actually_changes_the_value() {
        let data = [0x00, 0x00, 0x00, 0x01];

        assert_eq!(BinaryDecoder::new(&data, Endianness::Big).read_i32().unwrap(), 1);
        assert_eq!(BinaryDecoder::new(&data, Endianness::Little).read_i32().unwrap(), 0x01000000);
    }

    #[test]
    fn test_single_bytes_are_order_independent() {
        let data = [0xFF];

        assert_eq!(BinaryDecoder::new(&data, Endianness::Big).read_u8().unwrap(), 255);
        assert_eq!(BinaryDecoder::new(&data, Endianness::Little).read_u8().unwrap(), 255);
        assert_eq!(BinaryDecoder::new(&data, Endianness::Big).read_i8().unwrap(), -1);
    }

    #[test]
    fn test_values_are_read_in_sequence() {
        let data = [0x00, 0x01, 0x00, 0x02, 0x07];
        let mut decoder = BinaryDecoder::new(&data, Endianness::Big);

        assert_eq!(decoder.read_u16().unwrap(), 1);
        assert_eq!(decoder.read_u16().unwrap(), 2);
        assert_eq!(decoder.read_u8().unwrap(), 7);
    }

    #[test]
    fn test_end_of_data_is_an_error() {
        let data = [0x00, 0x01];
        let mut decoder = BinaryDecoder::new(&data, Endianness::Big);

        assert!(decoder.read_i32().is_err());
    }

    #[test]
    fn test_double_round_trips() {
        let expected = -12.5f64;
        let big = expected.to_be_bytes();
        let little = expected.to_le_bytes();

        assert_eq!(BinaryDecoder::new(&big, Endianness::Big).read_f64().unwrap(), expected);
        assert_eq!(BinaryDecoder::new(&little, Endianness::Little).read_f64().unwrap(), expected);
    }
}
