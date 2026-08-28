/// Represents the possible data types for properties in a PLY file.
/// Each variant corresponds to both traditional naming
/// and bit-width naming conventions (e.g. Int8 matches "char" and "int8").
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlyDataType {
    Int8,    // signed 8-bit integer
    UInt8,   // unsigned 8-bit integer
    Int16,   // signed 16-bit integer
    UInt16,  // unsigned 16-bit integer
    Int32,   // signed 32-bit integer
    UInt32,  // unsigned 32-bit integer
    Float32, // 32-bit floating point
    Float64, // 64-bit floating point
}

impl std::str::FromStr for PlyDataType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "char" => Ok(PlyDataType::Int8),
            "uchar" => Ok(PlyDataType::UInt8),
            "short" => Ok(PlyDataType::Int16),
            "ushort" => Ok(PlyDataType::UInt16),
            "int" => Ok(PlyDataType::Int32),
            "uint" => Ok(PlyDataType::UInt32),
            "float" => Ok(PlyDataType::Float32),
            "double" => Ok(PlyDataType::Float64),
            "int8" => Ok(PlyDataType::Int8),
            "uint8" => Ok(PlyDataType::UInt8),
            "int16" => Ok(PlyDataType::Int16),
            "uint16" => Ok(PlyDataType::UInt16),
            "int32" => Ok(PlyDataType::Int32),
            "uint32" => Ok(PlyDataType::UInt32),
            "float32" => Ok(PlyDataType::Float32),
            "float64" => Ok(PlyDataType::Float64),
            _ => Err(format!("Unknown PLY data type: {}", s)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::PlyDataType;

    #[test]
    fn test_datatype_from_string_parsing() {
        assert_eq!("float".parse::<PlyDataType>().unwrap(), PlyDataType::Float32);
        assert_eq!("int32".parse::<PlyDataType>().unwrap(), PlyDataType::Int32);
        assert_eq!("uchar".parse::<PlyDataType>().unwrap(), PlyDataType::UInt8);

        assert!("invalid".parse::<PlyDataType>().is_err());
    }
}
