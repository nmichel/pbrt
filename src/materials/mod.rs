mod diffuse_light;

pub mod dielectric;
pub mod lambertian;
pub mod material;
pub mod metal;
pub use self::dielectric::Dielectric;
pub use self::diffuse_light::DiffuseLight;
pub use self::lambertian::Lambertian;
pub use self::material::Material;
pub use self::metal::Metal;
