/// The encoding of a PLY file's data section, as declared by its `format` header line.
///
/// Reference: Greg Turk, *The PLY Polygon File Format* — the `format` line is mandatory and must
/// be the second line of the file, right after the `ply` magic.
///
/// The header itself is always ASCII, whatever the variant: the encoding announced here applies
/// only to the bytes that follow `end_header`.
///
/// The two binary variants differ solely by the byte order used for every multi-byte scalar. PLY
/// stores no byte-order mark, so the header line is the only thing that tells them apart — reading
/// a big-endian file as little-endian is silent, and produces garbage coordinates rather than an
/// error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlyFormat {
    Ascii,
    BinaryLittleEndian,
    BinaryBigEndian,
}

impl std::str::FromStr for PlyFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "ascii" => Ok(PlyFormat::Ascii),
            "binary_little_endian" => Ok(PlyFormat::BinaryLittleEndian),
            "binary_big_endian" => Ok(PlyFormat::BinaryBigEndian),
            _ => Err(format!("Unknown PLY format: {}", s)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::PlyFormat;

    #[test]
    fn test_format_from_string_parsing() {
        assert_eq!("ascii".parse::<PlyFormat>().unwrap(), PlyFormat::Ascii);
        assert_eq!("binary_little_endian".parse::<PlyFormat>().unwrap(), PlyFormat::BinaryLittleEndian);
        assert_eq!("binary_big_endian".parse::<PlyFormat>().unwrap(), PlyFormat::BinaryBigEndian);

        assert!("binary".parse::<PlyFormat>().is_err());
    }
}
