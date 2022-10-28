use super::Camera;
use crate::geom::ray::Ray;
use crate::geom::matrix4::Matrix4;
use crate::geom::vector2::Vector2u;
use crate::geom::vector3::Vector3f;

/// A naive camera implementation
pub struct PinHoleCamera {
  /// Transform a pixel coordinate to the corresponding point in screen space
  raster_to_screen: Matrix4,

  /// Transform from camera space to world space
  cam_to_world: Matrix4
}

impl PinHoleCamera {
  pub fn new(resolution: &Vector2u, fov: f64, near: f64, far: f64, cam_to_world: Matrix4) -> Self {
      let image_width = resolution.x;
      let image_height = resolution.y;
      let image_aspect_ratio = (image_width as f64) / (image_height as f64);

      let (screen_p_min_x, screen_p_max_x, screen_p_min_y, screen_p_max_y) = 
          if image_aspect_ratio > 1.0 {
              (-image_aspect_ratio, image_aspect_ratio, -1.0,  1.0)
          } else {
              (-1.0, 1.0, -1.0 / image_aspect_ratio, 1.0 / image_aspect_ratio)
          };

      let trans =
          Matrix4::perspective(fov, near, far).inverse() *
          Matrix4::translation(screen_p_min_x, screen_p_max_y, 0.0) *
          Matrix4::scale(screen_p_max_x - screen_p_min_x, screen_p_min_y - screen_p_max_y, 1.0) *
          Matrix4::scale(1.0 / (image_width as f64), 1.0 / (image_height as f64), 1.0);

      Self { raster_to_screen: trans, cam_to_world }
  }
}

impl Camera for PinHoleCamera {
  fn get_ray(&self, pixel_x: f64, pixel_y: f64) -> Ray {
      let pixel3d = Vector3f::new(pixel_x, pixel_y, 0.0);
      let mut camera_vector = &self.raster_to_screen * &pixel3d;
      camera_vector.normalize();
      Ray::new(&self.cam_to_world.transform_point(&Vector3f::new(0.0, 0.0, 0.0)), &self.cam_to_world.transform_direction(&camera_vector))
  }
}
