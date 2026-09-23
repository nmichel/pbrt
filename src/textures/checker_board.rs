use super::Texture;
use crate::geom::surface_point::SurfacePoint;
use crate::spectrum::Spectrum;

pub struct CheckerBoard {
    c1: Spectrum,
    c2: Spectrum,
    scale: f64,
}

impl CheckerBoard {
    pub fn new(c1: Spectrum, c2: Spectrum, scale: f64) -> Self {
        Self { c1, c2, scale }
    }
}

impl Texture for CheckerBoard {
    fn shade(&self, surface_point: &SurfacePoint) -> Spectrum {
        let scale_u = (surface_point.u.abs() * self.scale) % 1.0;
        let scale_v = (surface_point.v.abs() * self.scale) % 1.0;
        let use_color = (scale_u < 0.5 && scale_v < 0.5) || (scale_u >= 0.5 && scale_v >= 0.5);
        let positive_uv_prod = (surface_point.u.signum() * surface_point.v.signum()) >= 0.0;
        if use_color ^ positive_uv_prod {
            self.c1
        }
        else {
            self.c2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::geom::vector3::Vector3f;

    const SCALE: f64 = 2.0;

    /// A cell is `1 / (2 · scale)` across: the comparison is against a half period of `|u| · scale`
    /// taken modulo one, so the colour turns over twice per unit of that product.
    const CELL: f64 = 1.0 / (2.0 * SCALE);

    fn checker_board() -> CheckerBoard {
        CheckerBoard::new(Spectrum::new(1.0, 0.0, 0.0), Spectrum::new(0.0, 1.0, 0.0), SCALE)
    }

    /// The pattern is a function of (u, v) alone, so the rest of the point is arbitrary — but it is
    /// *given*, rather than left at zero, so that a texture starting to read `p` or `n` would be
    /// reading something.
    fn at(u: f64, v: f64) -> SurfacePoint {
        SurfacePoint {
            p: Vector3f::new(0.3, -0.4, 0.5),
            n: Vector3f::new(0.0, 1.0, 0.0),
            u,
            v,
        }
    }

    /// Stepping one cell along `u` changes the colour, and stepping two brings the first one back.
    ///
    /// This is the whole of what a checker board claims to be, and it is stated as a walk rather
    /// than as three expected colours: which of `c1` and `c2` a given cell carries is an accident of
    /// where the origin falls, and nothing should depend on it.
    #[test]
    fn test_the_colour_turns_over_once_per_cell() {
        let texture = checker_board();
        let v = CELL / 2.0;

        let first = texture.shade(&at(CELL / 2.0, v));
        let second = texture.shade(&at(CELL / 2.0 + CELL, v));
        let third = texture.shade(&at(CELL / 2.0 + 2.0 * CELL, v));

        assert_ne!(first, second, "neighbouring cells along u must differ");
        assert_eq!(first, third, "the pattern must repeat every two cells");
    }

    /// The same, along `v` — the two coordinates play the same role, and a texture that alternated
    /// along one axis only would be a set of stripes.
    #[test]
    fn test_the_colour_turns_over_once_per_cell_along_v() {
        let texture = checker_board();
        let u = CELL / 2.0;

        let first = texture.shade(&at(u, CELL / 2.0));
        let second = texture.shade(&at(u, CELL / 2.0 + CELL));

        assert_ne!(first, second, "neighbouring cells along v must differ");
    }

    /// Diagonal neighbours share a colour, which is what makes the pattern a checker board rather
    /// than a grid: stepping one cell along *both* axes turns the colour over twice.
    #[test]
    fn test_diagonal_neighbours_share_a_colour() {
        let texture = checker_board();

        let origin = texture.shade(&at(CELL / 2.0, CELL / 2.0));
        let diagonal = texture.shade(&at(CELL / 2.0 + CELL, CELL / 2.0 + CELL));

        assert_eq!(origin, diagonal);
    }

    /// Crossing `u = 0` turns the colour over, exactly as crossing any other cell boundary does.
    ///
    /// This is what the product of the signs is for, and the property is easiest to see through
    /// what its absence would give: the cell index is read from `|u|`, so `−u` and `+u` fall in the
    /// same cell and would take the same colour. Two cells of one colour would then meet along the
    /// line `u = 0`, and the pattern would be mirrored about the axes instead of continued across
    /// them.
    #[test]
    fn test_the_pattern_continues_across_the_origin() {
        let texture = checker_board();
        let v = CELL / 2.0;

        let right_of_the_axis = texture.shade(&at(CELL / 2.0, v));
        let left_of_the_axis = texture.shade(&at(-CELL / 2.0, v));

        assert_ne!(
            right_of_the_axis, left_of_the_axis,
            "the cells either side of u = 0 are neighbours, so they must differ"
        );
    }
}
