mod cameras;
mod materials;
mod objects;
mod scene;
mod shapes;
mod stage;
mod textures;
mod transform;

pub use self::cameras::*;
pub use self::materials::*;
pub use self::objects::*;
pub use self::scene::*;
pub use self::shapes::*;
pub use self::stage::*;
pub use self::textures::*;
pub use self::transform::*;

use super::visitors::Visitor;

pub trait Node {
    fn visit(self: &Self, visitor: &mut dyn Visitor);
}
