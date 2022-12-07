pub mod integrator;
pub mod normal;
pub mod path;
// pub mod whitted;

pub use self::integrator::Integrator;
pub use self::normal::NormalIntegrator;
pub use self::path::PathIntegrator;

#[derive(Debug)]
pub enum Type {
  PATH,
  NORMAL
}
