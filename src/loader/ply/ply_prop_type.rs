/// Represents the properties of elements in a PLY file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementProps {
    VertexX,
    VertexY,
    VertexZ,
    VertexColorRed,
    VertexColorGreen,
    VertexColorBlue,
    VertexColorAlpha,
    VertexNormalX,
    VertexNormalY,
    VertexNormalZ,
    VertexTexCoordU,
    VertexTexCoordV,
    FaceVertexIndices,
    Unknown(String),
}

impl std::str::FromStr for ElementProps {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "x" => Ok(ElementProps::VertexX),
            "y" => Ok(ElementProps::VertexY),
            "z" => Ok(ElementProps::VertexZ),
            "red" => Ok(ElementProps::VertexColorRed),
            "green" => Ok(ElementProps::VertexColorGreen),
            "blue" => Ok(ElementProps::VertexColorBlue),
            "alpha" => Ok(ElementProps::VertexColorAlpha),
            "nx" => Ok(ElementProps::VertexNormalX),
            "ny" => Ok(ElementProps::VertexNormalY),
            "nz" => Ok(ElementProps::VertexNormalZ),
            "u" => Ok(ElementProps::VertexTexCoordU),
            "s" => Ok(ElementProps::VertexTexCoordU),
            "v" => Ok(ElementProps::VertexTexCoordV),
            "t" => Ok(ElementProps::VertexTexCoordV),
            "vertex_index" | "vertex_indices" => Ok(ElementProps::FaceVertexIndices),
            _ => Ok(ElementProps::Unknown(s.to_string())),
        }
    }
}

#[cfg(test)]
mod test {
    use super::ElementProps;

    #[test]
    fn test_element_props_from_str() {
        assert_eq!("x".parse::<ElementProps>().unwrap(), ElementProps::VertexX);
        assert_eq!("y".parse::<ElementProps>().unwrap(), ElementProps::VertexY);
        assert_eq!("z".parse::<ElementProps>().unwrap(), ElementProps::VertexZ);
        assert_eq!("red".parse::<ElementProps>().unwrap(), ElementProps::VertexColorRed);
        assert_eq!("green".parse::<ElementProps>().unwrap(), ElementProps::VertexColorGreen);
        assert_eq!("blue".parse::<ElementProps>().unwrap(), ElementProps::VertexColorBlue);
        assert_eq!("alpha".parse::<ElementProps>().unwrap(), ElementProps::VertexColorAlpha);
        assert_eq!("nx".parse::<ElementProps>().unwrap(), ElementProps::VertexNormalX);
        assert_eq!("ny".parse::<ElementProps>().unwrap(), ElementProps::VertexNormalY);
        assert_eq!("nz".parse::<ElementProps>().unwrap(), ElementProps::VertexNormalZ);
        assert_eq!("u".parse::<ElementProps>().unwrap(), ElementProps::VertexTexCoordU);
        assert_eq!("s".parse::<ElementProps>().unwrap(), ElementProps::VertexTexCoordU);
        assert_eq!("v".parse::<ElementProps>().unwrap(), ElementProps::VertexTexCoordV);
        assert_eq!("t".parse::<ElementProps>().unwrap(), ElementProps::VertexTexCoordV);
        assert_eq!("vertex_index".parse::<ElementProps>().unwrap(), ElementProps::FaceVertexIndices);
        assert_eq!("vertex_indices".parse::<ElementProps>().unwrap(), ElementProps::FaceVertexIndices);
        assert_eq!("unknown".parse::<ElementProps>().unwrap(), ElementProps::Unknown("unknown".to_string()));
    }
}
