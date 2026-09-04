use super::Camera;
use crate::geom::matrix4::Matrix4;
use crate::geom::ray::Ray;
use crate::geom::vector2::{Vector2f, Vector2u};
use crate::geom::vector3::Vector3f;
use crate::utils::random_double;
use std::f64::consts::PI;

/// A uniform sample of the unit disk, drawn from a sample of the unit square.
///
/// Reference: PBR Book, 3ed, §13.6.2 — *Sampling a Unit Disk*.
/// <https://www.pbr-book.org/3ed-2018/Monte_Carlo_Integration/2D_Sampling_with_Multidimensional_Transformations>
///
/// **Frame**: `u` is a point of [0,1)², the result a point of the disk of radius 1 centred on the
/// origin. The lens lies in the plane z = 0 of camera space, so the two coordinates are (x, y).
///
/// Uniform *per unit area* is the requirement, and it is what makes the obvious
/// `(r, θ) = (ξ₁, 2πξ₂)` wrong: equal ranges of r cover unequal areas, so that map would crowd
/// samples towards the centre. Writing the target density and pushing it through the change of
/// variables shows where the correction comes from:
///
///   p(x, y) = 1/π                                     [1]  uniform over an area of π
///   p(r, θ) = r ⋅ p(x, y) = r/π                       [2]  |∂(x,y)/∂(r,θ)| = r
///
/// Split [2] into a marginal in r and a conditional in θ:
///
///   p(r)   = ∫₀^2π (r/π) dθ = 2r                      [3]
///   p(θ|r) = p(r, θ) / p(r) = 1/(2π)                  [4]
///
/// [4] does not depend on r, so θ is uniform *and* independent of the radius: the two components
/// of `u` can be inverted separately. Integrating [3] and inverting each cdf:
///
///   P(r) = ∫₀ʳ 2r' dr' = r²    ⟹   r = √ξ₁           [5]
///   P(θ) = θ / (2π)            ⟹   θ = 2π ξ₂         [6]
///
/// The √ of [5] is the whole point — it pushes samples outwards by exactly what the r of [2] takes
/// away. Two draws, no rejection, so the cost of a lens sample does not depend on the draw.
///
/// pbrt prefers a *concentric* map here, which reaches the same distribution while distorting the
/// unit square less; the difference is invisible as long as the two components of `u` are drawn
/// independently of each other.
fn sample_uniform_disk(u: &Vector2f) -> Vector2f {
    let r = u.x.sqrt(); // [5]
    let theta = 2.0 * PI * u.y; // [6]
    Vector2f::new(r * theta.cos(), r * theta.sin())
}

/// A simple thins len camera implementation
pub struct ThinLensCamera {
    /// Transform a pixel coordinate to the corresponding point in screen space
    raster_to_screen: Matrix4,

    /// Transform from camera space to world space
    cam_to_world: Matrix4,

    lens_radius: f64,

    focal_distance: f64,
}

impl ThinLensCamera {
    pub fn new(resolution: &Vector2u, fov: f64, near: f64, far: f64, lens_radius: f64, focal_distance: f64, cam_to_world: Matrix4) -> Self {
        let image_width = resolution.x;
        let image_height = resolution.y;
        let image_aspect_ratio = (image_width as f64) / (image_height as f64);

        let screen_p_min_x;
        let screen_p_max_x;
        let screen_p_min_y;
        let screen_p_max_y;

        if image_aspect_ratio > 1.0 {
            screen_p_min_x = -image_aspect_ratio;
            screen_p_max_x = image_aspect_ratio;
            screen_p_min_y = -1.0;
            screen_p_max_y = 1.0;
        }
        else {
            screen_p_min_x = -1.0;
            screen_p_max_x = 1.0;
            screen_p_min_y = -1.0 / image_aspect_ratio;
            screen_p_max_y = 1.0 / image_aspect_ratio;
        }

        let trans = Matrix4::perspective(fov, near, far).inverse()
            * Matrix4::translation(screen_p_min_x, screen_p_max_y, 0.0)
            * Matrix4::scale(screen_p_max_x - screen_p_min_x, screen_p_min_y - screen_p_max_y, 1.0)
            * Matrix4::scale(1.0 / (image_width as f64), 1.0 / (image_height as f64), 1.0);

        Self {
            lens_radius,
            focal_distance,
            raster_to_screen: trans,
            cam_to_world,
        }
    }
}

impl Camera for ThinLensCamera {
    fn get_ray(&self, pixel_x: f64, pixel_y: f64) -> Ray {
        let pixel3d = Vector3f::new(pixel_x, pixel_y, 0.0);
        let mut camera_vector = &self.raster_to_screen * &pixel3d;
        camera_vector.normalize();

        let ray = Ray::new(&Vector3f::new(0.0, 0.0, 0.0), &camera_vector);

        let lens_sample = Vector2f::new(random_double(), random_double());
        let pixel_lens = sample_uniform_disk(&lens_sample) * self.lens_radius;
        let ft = self.focal_distance / camera_vector.z;

        let origin = Vector3f::new(pixel_lens.x, pixel_lens.y, 0.0);
        let mut dir = ray.point_at(ft) - origin;
        dir.normalize();

        Ray::new(&self.cam_to_world.transform_point(&origin), &self.cam_to_world.transform_direction(&dir))
    }
}
