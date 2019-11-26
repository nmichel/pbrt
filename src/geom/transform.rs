use super::matrix4::Matrix4;
use super::vector3::Vector3f;

pub struct Transform {
    mat: Matrix4,
    inv_mat: Matrix4,
}

impl Transform {
    pub fn from_matrix(mat: Matrix4) -> Self {
        let inv_mat = mat.inverse();

        Self {
            mat,
            inv_mat: inv_mat
        }
    }

    pub fn from_matrix_and_inverse(mat: Matrix4, inv_mat: Matrix4) -> Self {
        Self {  
            mat,
            inv_mat
        }
    }

    pub fn translation(p: Vector3f) -> Self {
        Self {
            mat: Matrix4::translation(p.x, p.y, p.z),
            inv_mat: Matrix4::translation(-p.x, -p.y, -p.z)
        }
    }

    pub fn transform_point_to_world(&self, p: &Vector3f) -> Vector3f {
        self.mat.transform_point(&p)
    }

    pub fn transform_direction_to_world(&self, p: &Vector3f) -> Vector3f {
        self.mat.transform_direction(&p)
    }

    pub fn transform_normal_to_world(&self, p: &Vector3f) -> Vector3f {
        self.mat.transform_normal(&p)
    }

    pub fn transform_point_to_local(&self, p: &Vector3f) -> Vector3f {
        self.inv_mat.transform_point(&p)
    }

    pub fn transform_direction_to_local(&self, p: &Vector3f) -> Vector3f {
        self.inv_mat.transform_direction(&p)
    }

    pub fn transform_normal_to_local(&self, p: &Vector3f) -> Vector3f {
        self.inv_mat.transform_normal(&p)
    }
}