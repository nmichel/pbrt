use crate::geom::aabound::AABound;
use crate::geom::intersectable::Intersectable;
use crate::geom::surface_point::SurfacePoint;
use crate::geom::vector2::Vector2f;

pub trait Shape: Intersectable + AABound {}

/// A point drawn on a surface, and the density it was drawn with.
pub struct ShapeSample {
    /// Where the point is, and its surface parameters — so that whoever reads a texture there
    /// reads the one the visible surface shows, at the same point.
    pub sp: SurfacePoint,

    /// Density **in area measure**: 1/area for a uniform draw, and it integrates to one over the
    /// surface rather than over directions. Converting it to a solid angle needs the point the
    /// surface is being looked at *from*, which a shape knows nothing about, so that conversion
    /// belongs to whoever holds both.
    pub pdf: f64,
}

/// A surface that can hand out points of itself, drawn uniformly over its area.
///
/// Reference: PBR Book, 3ed, chapter 14 — *Light Transport I*, « Sampling Light Sources ».
/// <https://www.pbr-book.org/3ed-2018/Light_Transport_I_Surface_Reflection/Sampling_Light_Sources>
///
/// # Why a trait apart from `Shape`
///
/// Not every shape has an area to draw from. A [`Plane`] is unbounded, and the area of a
/// [`csg::Union`](csg::Union) is not a quantity its members can compute — inclusion-exclusion over
/// arbitrary intersections is not something the operator knows how to do. Were this on `Shape`,
/// those would have to supply an implementation for a question they cannot answer, and the only
/// honest bodies would be a panic or a lie. A separate trait makes "can be sampled by area" a
/// property a shape has or has not, which is what it is.
///
/// # The numbers come from the caller
///
/// `sample_area` consumes a `u` rather than reaching for a sampler, exactly as
/// [`Pdf::generate`](crate::pdfs::Pdf::generate) does. Where the pair comes from is then the
/// caller's business, which is what keeps a render reproducible: the one place that decides how
/// numbers are drawn stays the one place that builds the sampler.
///
/// # Frame
///
/// The point comes out in the shape's own space, like every other quantity a shape exchanges. A
/// [`Transformed`] shape places it on the way out, as it places a hit.
pub trait AreaSampleable: Send + Sync {
    /// The total surface area, in the shape's own space.
    fn area(&self) -> f64;

    /// Draws a point of the surface from a pair of numbers in [0, 1)².
    fn sample_area(&self, u: &Vector2f) -> ShapeSample;
}

mod aabox;
pub mod csg;
mod cylinder;
mod plane;
mod rectangle;
mod sphere;
mod transformed;
mod triangle;
pub mod triangle_mesh;

pub use self::aabox::AABox;
pub use self::cylinder::Cylinder;
pub use self::plane::Plane;
pub use self::rectangle::Rectangle;
pub use self::sphere::Sphere;
pub use self::transformed::Transformed;
pub use self::triangle::Triangle;
pub use self::triangle_mesh::TriangleMesh;
