use super::geom::ray::Ray;
use super::geom::matrix4::Matrix4;
use super::geom::vector2::Vector2u;
use super::geom::vector3::Vector3f;

pub trait Camera {
    /// Returns the `Ray` passing through pixel at coordinates `pixel`
    fn get_ray(&self, pixel_x: f64, pixel_y: f64) -> Ray;
}

/// A naive camera implementation
pub struct PinHoleCamera {
    /// Transform a pixel coordinate to the corresponding point in screen space
    raster_to_screen: Matrix4
}

impl PinHoleCamera {
    pub fn new(resolution: &Vector2u, fov: f64, near: f64, far: f64) -> Self {
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
        } else {
            screen_p_min_x = -1.0;
            screen_p_max_x = 1.0;
            screen_p_min_y = -1.0 / image_aspect_ratio;
            screen_p_max_y = 1.0 / image_aspect_ratio;
        }

        let trans =
            Matrix4::perspective(fov, near, far).inverse() *
            Matrix4::translation(screen_p_min_x, screen_p_max_y, 0.0) *
            Matrix4::scale(screen_p_max_x - screen_p_min_x, screen_p_min_y - screen_p_max_y, 1.0) *
            Matrix4::scale(1.0 / (image_width as f64), 1.0 / (image_height as f64), 1.0);

        Self { raster_to_screen: trans }
    }
}

impl Camera for PinHoleCamera {
    fn get_ray(&self, pixel_x: f64, pixel_y: f64) -> Ray {
        let pixel3d = Vector3f::new(pixel_x, pixel_y, 0.0);
        let mut camera_vector = &self.raster_to_screen * &pixel3d;
        camera_vector.normalize();
        Ray::new(&Vector3f::new(0.0, 0.0, 0.0), &camera_vector)
    }
}
