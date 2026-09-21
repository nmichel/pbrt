//! Constructive solid geometry: a shape built from other shapes by set operations.
//!
//! Reference: PBR Book, 3ed, chapter 3 — *Shapes*, and the classic treatment in Roth,
//! *Ray Casting for Modeling Solids* (1982), which is where ray casting against a CSG tree comes
//! from.
//!
//! # What an element is
//!
//! **A `Shape`, and nothing more.** An operation holds `Vec<Arc<dyn Shape>>` and knows only how to
//! combine the boundary points its members report with the volumes they enclose. A piece that sits
//! somewhere inside the assembly is a [`Transformed`](super::Transformed), which is what a
//! `transform` block on an `elem` builds; a piece needing no placement is the bare shape. Neither
//! is the operation's business, and that is the point — a set operation on placed shapes is the
//! same set operation.
//!
//! # The two frames a CSG lives in, which do not compose
//!
//! An assembly is described in **its own frame** and placed in the world **once**:
//!
//! | | What it positions | Where it is applied |
//! |---|---|---|
//! | an `elem`'s transformation | a piece inside the assembly | on the element, here |
//! | the current transformation matrix | the assembly as a whole | on the finished shape, by the loader |
//!
//! So an `elem`'s `transform` block does **not** enter the loader's current transformation matrix.
//! Composing the two would place the assembly twice, and silently: the image would show the shape
//! somewhere no line of the description mentions.
//!
//! What this yields is nested placements —
//! `Transformed(Union(Transformed(sphere), Transformed(sphere)), ctm)` — and therefore two levels
//! of ray round trip. Exactly two, one per frame that genuinely exists.
//!
//! # What every operation owes its members
//!
//! `Intersectable::intersect` reports the **boundary** of the result and `contain_point` its
//! **interior**, and the two must agree: an operation reads its members' interiors to decide which
//! of their boundary points survive, so a member whose two answers disagree carves holes through
//! geometry. The convention is stated once, on
//! [`Intersectable::contain_point`](crate::geom::intersectable::Intersectable::contain_point) —
//! closed solids, and an open surface encloses nothing.

mod intersection;
mod substraction;
mod union;

pub use self::intersection::Intersection;
pub use self::substraction::Substraction;
pub use self::union::Union;
