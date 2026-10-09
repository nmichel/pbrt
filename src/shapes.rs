use crate::geom::aabound::AABound;
use crate::geom::intersectable::Intersectable;
use crate::geom::ray::Ray;
use crate::geom::surface_point::SurfacePoint;
use crate::geom::vector2::Vector2f;
use crate::geom::vector3;
use crate::geom::vector3::Vector3f;
use std::sync::Arc;

pub trait Shape: Intersectable + AABound {
    /// The area-sampling face of this shape, when it has one.
    ///
    /// Rust has no trait upcast, and a scene is assembled from `Arc<dyn Shape>` — the concrete type
    /// is gone exactly where it would have answered. Asking the shape keeps "can I be sampled by
    /// area" a property the shape states, rather than one a caller infers from a type it no longer
    /// has, and `Any` followed by a downcast would be the contrivance to avoid.
    ///
    /// The default is `None`, so a shape with no area to draw from — [`Plane`], the constructive
    /// operators of [`csg`] — says so by saying nothing. The receiver is owned because the answer
    /// is a handle the caller keeps; this is asked once per object, when a scene is built.
    fn area_sampler(self: Arc<Self>) -> Option<Arc<dyn AreaSampleable>> {
        None
    }
}

/// A point drawn on a surface, and the density it was drawn with.
pub struct ShapeSample {
    /// Where the point is, and its surface parameters — so that whoever reads a texture there
    /// reads the one the visible surface shows, at the same point.
    pub sp: SurfacePoint,

    /// Density **in area measure**: 1/area for a uniform draw, and it integrates to one over the
    /// surface rather than over directions. [`solid_angle_from_area`] is what converts it, and it
    /// needs the point the surface is looked at from.
    pub pdf: f64,
}

/// A point drawn on a surface **as seen from somewhere**: the direction to it, and the density of
/// having drawn that direction.
///
/// It is a type apart from [`ShapeSample`] for one reason, and it is not tidiness: the two carry a
/// `pdf` in two different measures, and a single field meaning either depending on which method
/// produced it is the kind of ambiguity this project has already paid to remove once — the `Option`
/// of `Material::emit` said both "does not emit" and "emits nothing this way" until the two were
/// separated.
pub struct SolidAngleSample {
    /// Where the point is, and its surface parameters, exactly as [`ShapeSample`] carries them.
    pub sp: SurfacePoint,

    /// Unit direction **from the reference point towards `sp.p`**.
    ///
    /// It comes back rather than being recomputed by the caller because the density below is the
    /// density of *this* direction: letting the two be derived separately invites them to disagree
    /// by a rounding, and lets a sign be lost in between.
    pub wi: Vector3f,

    /// Density **per unit solid angle at the reference point**, which is the measure an integrator
    /// divides by.
    pub pdf: f64,
}

/// Reads a draw made over an area as a draw made over directions, seen from `reference`.
///
/// Reference: PBR Book, 4ed, §4.2.3 — « Integrals over Area », equation (4.9).
/// <https://pbr-book.org/4ed/Radiometry,_Spectra,_and_Color/Working_with_Radiometric_Integrals#IntegralsoverArea>
///
/// # The change of measure
///
/// Write `p` for the reference point, `pₗ` for the point drawn, `nₗ` for the normal there,
/// `d = ‖pₗ − p‖`, and `θₗ` for the angle at the drawn point between `nₗ` and the direction back
/// towards `p`. A patch `dA` of the surface, seen from `p`, subtends
///
/// ```text
/// [1]  dω = dA · |cos θₗ| / d²
/// ```
///
/// — `cos θₗ` because the patch is foreshortened by its tilt away from `p`, and `1/d²` because a
/// fixed patch subtends less angle the further it sits. Densities transform by the reciprocal of
/// that jacobian, the same event having to carry the same probability under either measure:
///
/// ```text
/// [2]  p(ω) = p(A) · dA/dω = p(A) · d² / |cos θₗ|
/// ```
///
/// **The inverse-square law is [1], and nothing else.** It is already in the density, so a caller
/// that also divided a radiance by `d²` would darken its image by `d²` a second time. Radiance
/// along a ray does not fall off with distance; what falls off is the solid angle the surface
/// occupies, which is exactly what [2] accounts for.
///
/// **`|cos θₗ| → 0` makes the density explode**, and that is correct rather than a defect: a
/// surface seen edge-on subtends almost no solid angle, so a point drawn on it stands for a
/// vanishing fraction of the directions leaving `p`, and its contribution is divided by a large
/// number. The variance is real, and it is what drawing in solid angle directly removes.
///
/// # Why the cosine is under an absolute value here
///
/// This is geometry and nothing else: a direction has a density whichever face of the surface the
/// point was drawn on. **Whether that face emits is a separate question**, and it belongs to the
/// caller — a shape does not know that the surface wearing it is one-sided. A caller that drops
/// back-facing points does so after reading this, on the normal it is handed.
pub fn solid_angle_from_area(sample: ShapeSample, reference: &Vector3f) -> Option<SolidAngleSample> {
    let to_sample = &sample.sp.p - reference;
    let squared_distance = to_sample.squared_length();
    if squared_distance == 0.0 {
        return None;
    }

    let wi = to_sample.normalized();

    // `wi` points from the reference towards the surface and the cosine is stated the other way
    // round, but the absolute value makes the two readings the same number.
    let cos_theta_l = vector3::abs_dot(&sample.sp.n, &wi);
    let pdf = solid_angle_density(sample.pdf, squared_distance, cos_theta_l)?;

    Some(SolidAngleSample { sp: sample.sp, wi, pdf })
}

/// `[2]` of [`solid_angle_from_area`], `p(ω) = p(A) · d² / |cos θₗ|`, and the values it refuses.
///
/// A density that comes out infinite — the surface seen edge-on, `cos θₗ = 0` — or not positive
/// cannot be divided by, so the sample is dropped. Dropping a sample is a decision about the
/// *distribution*: the directions it stood for are never drawn, so their density is 0, and
/// whoever evaluates the density afterwards must say so. Drawing and evaluating both come through
/// here — [`solid_angle_from_area`] and [`solid_angle_pdf_from_area`] — and so cannot disagree on
/// what was thrown away.
fn solid_angle_density(area_pdf: f64, squared_distance: f64, abs_cos_theta_l: f64) -> Option<f64> {
    let pdf = area_pdf * squared_distance / abs_cos_theta_l;
    if !pdf.is_finite() || pdf <= 0.0 {
        return None;
    }
    Some(pdf)
}

/// A surface that can hand out points of itself, drawn uniformly over its area.
///
/// Reference: PBR Book, 4ed, §6.1.7 — *Shapes*, « Sampling ».
/// <https://pbr-book.org/4ed/Shapes/Basic_Shape_Interface#Sampling>
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
///
/// # Why it requires `Intersectable`
///
/// A shape that hands out points of itself must also answer the converse: given a direction, how
/// likely was it to have been drawn — which first means finding the point of the surface that
/// direction reaches. [`pdf_solid_angle`](Self::pdf_solid_angle) does that by intersecting the
/// shape, so its default body needs the shape to be intersectable. Every shape that can be sampled
/// already is, since being seen is what makes a surface worth lighting from.
pub trait AreaSampleable: Intersectable {
    /// The total surface area, in the shape's own space.
    fn area(&self) -> f64;

    /// Draws a point of the surface from a pair of numbers in [0, 1)².
    fn sample_area(&self, u: &Vector2f) -> ShapeSample;

    /// Draws a point of the surface **as seen from `reference`**, with a density over directions.
    ///
    /// Reference: PBR Book, 4ed, §6.1.7 — *Shapes*, « Sampling », which keeps the same two
    /// variants and for the same reason.
    /// <https://pbr-book.org/4ed/Shapes/Basic_Shape_Interface#Sampling>
    ///
    /// # Why a second method rather than a replacement
    ///
    /// Because an integrator works in solid angle and a surface is parameterised by area, and the
    /// bridge between them can be crossed in two places. The default body crosses it *after* the
    /// draw, converting with [`solid_angle_from_area`]: it is correct for every shape, costs
    /// nothing to adopt, and is what every implementation gets until it asks for better. A shape
    /// that knows the solid angle it subtends can instead draw *within* it, and spend every one of
    /// its samples where the light actually comes from.
    ///
    /// The two are not interchangeable even then. Drawing by area stays the honest answer to "hand
    /// me a point of yourself", which is a question with no reference point in it; and it remains
    /// the fallback when there is no cone to speak of, as there is none for a point inside a
    /// closed surface.
    ///
    /// # Frame, and what `None` means
    ///
    /// `reference` is in the shape's own space, like everything else a shape exchanges, and so is
    /// the point that comes back. [`Transformed`] carries both across its placement.
    ///
    /// `None` is "there is no direction to speak of" — the reference point lies on the surface, or
    /// the density came out as something that cannot be divided by. It is **not** "nothing is
    /// emitted that way": a shape knows nothing of emission.
    fn sample_solid_angle(&self, reference: &Vector3f, u: &Vector2f) -> Option<SolidAngleSample> {
        solid_angle_from_area(self.sample_area(u), reference)
    }

    /// The density with which [`sample_solid_angle`](Self::sample_solid_angle), called from
    /// `reference`, would have produced the direction `wi`.
    ///
    /// Reference: PBR Book, 4ed, §6.1.7 — *Shapes*, « Sampling », `Shape::PDF`.
    /// <https://pbr-book.org/4ed/Shapes/Basic_Shape_Interface#Sampling>
    ///
    /// # The contract
    ///
    /// For every direction `sample_solid_angle` can produce from `reference`, this returns the
    /// density that sample came with; for every other direction, 0. The two methods are a pair:
    /// **a shape that overrides one overrides the other**, and so does any wrapper that forwards
    /// one. Otherwise the default below answers for a draw that is not the one made, and the
    /// weights multiple importance sampling builds on these densities stop adding up to 1.
    ///
    /// **"The sample" is the one at the first point `wi` meets.** A closed surface drawn by area
    /// yields points on its far side too, along directions that also reach its near side; this
    /// answers for the near one, which is the point a ray along `wi` sees and the only one a
    /// one-sided emitter keeps. Its integral over directions is then the share of draws that land
    /// on the near side — `(1 − r/d)/2` for a sphere seen from distance `d` — and not 1.
    ///
    /// # The default
    ///
    /// [`solid_angle_pdf_from_area`]: the density a draw by area converted into solid angle gives
    /// `wi`, which is what the default [`sample_solid_angle`](Self::sample_solid_angle) draws with.
    ///
    /// # Frame
    ///
    /// As for `sample_solid_angle`: `reference` in the shape's own space, `wi` a unit vector from it.
    fn pdf_solid_angle(&self, reference: &Vector3f, wi: &Vector3f) -> f64 {
        solid_angle_pdf_from_area(self, reference, wi)
    }
}

/// The density with which [`solid_angle_from_area`], applied to a uniform draw over `shape`, would
/// have produced `wi` from `reference` — the evaluation that undoes that draw.
///
/// It finds the point the direction reaches by intersecting the shape, then reads `[2]` of
/// [`solid_angle_from_area`] at that point, with the area density `1/area` that a uniform draw has
/// everywhere.
///
/// - **No hit**: the direction misses the surface, which a draw of a point on it never produces —
///   density 0.
/// - **The first hit**, not any other: it is the point a ray along `wi` sees. A closed surface
///   drawn by area also yields points on its far side, along the same directions; those face away
///   from `reference`, and the one-sided emitters this serves drop them.
/// - **A refused density**, edge-on or not positive, is 0: `solid_angle_density` refuses the same
///   values here as when drawing.
///
/// **The cost**: one intersection with this shape alone, paid by whoever asks — a bsdf ray that has
/// just reached the source, and has already paid for the traversal of the whole scene.
///
/// A free function rather than only the trait's default body, for the reason
/// [`solid_angle_from_area`] is one: a shape that draws better than by area still falls back to it
/// somewhere, and has to evaluate that fallback the same way.
pub fn solid_angle_pdf_from_area<S: AreaSampleable + ?Sized>(shape: &S, reference: &Vector3f, wi: &Vector3f) -> f64 {
    let ray = Ray::new(reference, wi);
    let hits = shape.intersect(&ray, 0.0, f64::MAX);
    let Some(hit) = hits.first()
    else {
        return 0.0;
    };

    let squared_distance = (&hit.p - reference).squared_length();
    let cos_theta_l = vector3::abs_dot(&hit.n, wi);
    solid_angle_density(1.0 / shape.area(), squared_distance, cos_theta_l).unwrap_or(0.0)
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
