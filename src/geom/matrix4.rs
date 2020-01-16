use std::f64;
use std::fmt;
use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, Div};
use super::vector3;
use super::vector3::Vector3f;

/// A 3D vector generic type.
/// 
#[derive(Debug, PartialEq)]
pub struct Matrix4 {
    m: [[f64; 4]; 4]
}

impl fmt::Display for Matrix4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[").ok();
        for i in  0..4 {
            write!(f, "[{} {} {} {}]", self.m[i][0], self.m[i][1], self.m[i][2], self.m[i][3]).ok();
        }
        write!(f, "]")
    }
}

impl Matrix4 {
    pub fn zero() -> Self {
        Self {
            m: [[0.0; 4]; 4]
        }
    }

    pub fn identity() -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0]
            ]
        }
    }

    pub fn translation(x: f64, y: f64, z: f64) -> Self {
        Self {
            m: [
                [1.0, 0.0, 0.0, x],
                [0.0, 1.0, 0.0, y],
                [0.0, 0.0, 1.0, z],
                [0.0, 0.0, 0.0, 1.0]
            ]
        }
    }

    pub fn scale(x: f64, y: f64, z: f64) -> Self {
        Self {
            m: [
                [  x, 0.0, 0.0, 0.0],
                [0.0,   y, 0.0, 0.0],
                [0.0, 0.0,   z, 0.0],
                [0.0, 0.0, 0.0, 1.0]
            ]
        }
    }

    pub fn rotation_x(theta: f64) -> Self {
        let s = theta.sin();
        let c = theta.cos();
        Self {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0,   c,  -s, 0.0],
                [0.0,   s,   c, 0.0],
                [0.0, 0.0, 0.0, 1.0]
            ]
        }
    }

    pub fn rotation_y(theta: f64) -> Self {
        let s = theta.sin();
        let c = theta.cos();
        Self {
            m: [
                [  c, 0.0,   s, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [ -s, 0.0,   c, 0.0],
                [0.0, 0.0, 0.0, 1.0]
            ]
        }
    }

    pub fn rotation_z(theta: f64) -> Self {
        let s = theta.sin();
        let c = theta.cos();
        Self {
            m: [
                [  c,  -s, 0.0, 0.0],
                [  s,   c, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0]
            ]
        }
    }

    pub fn perspective(fov: f64, n: f64, f: f64) -> Self {
        let s = 1.0 / (fov / 2.0).tan();
        Self {
            m: [
                [  s, 0.0,         0.0,               0.0],
                [0.0,   s,         0.0,               0.0],
                [0.0, 0.0, f / (f - n),  -f * n / (f - n)],
                [0.0, 0.0,         1.0,               0.0]
            ]
        }
    }

    pub fn look_at(pos: &Vector3f, look: &Vector3f, up: &Vector3f) -> Self {
        let mut dir = look - pos;
        dir.normalize();
        let mut norm_up: Vector3f = *up;
        norm_up.normalize();
        let mut right = vector3::cross(&norm_up, &dir);
        right.normalize();
        let real_up = vector3::cross(&dir, &right);

        Self {
            m: [
                [right.x, real_up.x, dir.x, pos.x],
                [right.y, real_up.y, dir.y, pos.y],
                [right.z, real_up.z, dir.z, pos.z],
                [    0.0,     0.0,    0.0,   1.0],
            ]
        }
    }

    pub fn inverse(&self) -> Self {
        let mut indxc: [usize; 4] = [0, 0, 0, 0];
        let mut indxr: [usize; 4] = [0, 0, 0, 0];
        let mut ipiv: [usize; 4] = [0, 0, 0, 0];
        let mut minv: [[f64; 4]; 4] = self.m;

        for i in 0..4 {
            let mut irow = 0;
            let mut icol = 0;
            let mut big = 0.0;

            for j in 0..4 {
                if ipiv[j] != 1 {
                    for k in 0..4 {
                        if ipiv[k] == 0 {
                            if (minv[j][k]).abs() >= big {
                                big = (minv[j][k]).abs();
                                irow = j;
                                icol = k;
                            }
                        } else if ipiv[k] > 1 {
                            // Error("Singular matrix in MatrixInvert");
                        }
                    }
                }
            }

            ipiv[icol] += 1;
            // Swap rows _irow_ and _icol_ for pivot
            if irow != icol {
                for k in 0..4 {
                    let tmp = minv[irow][k];
                    minv[irow][k] = minv[icol][k];
                    minv[icol][k] = tmp;
                }
            }
            indxr[i] = irow;
            indxc[i] = icol;
            if minv[icol][icol] == 0.0 {
                // Error("Singular matrix in MatrixInvert");
            }

            // Set $m[icol][icol]$ to one by scaling row _icol_ appropriately
            let pivinv = 1.0 / minv[icol][icol];
            minv[icol][icol] = 1.0;
            for j in 0..4 {
                minv[icol][j] *= pivinv;
            }

            // Subtract this row from others to zero out their columns
            for j in 0..4 {
                if j != icol {
                    let save = minv[j][icol];
                    minv[j][icol] = 0.0;
                    for k in 0..4 {
                        minv[j][k] -= minv[icol][k] * save;
                    }
                }
            }
        }

        // Swap columns to reflect permutation
        for j in (0..4).rev() {
            if indxr[j] != indxc[j] {
                for k in 0..4 {
                    let tmp = minv[k][indxr[j]];
                    minv[k][indxr[j]] = minv[k][indxc[j]];
                    minv[k][indxc[j]] = tmp;
                }
            }
        }

        Self {
            m: minv
        }
    }

    pub fn transpose(&self) -> Self {
        Self {
            m: [
                [self.m[0][0], self.m[1][0], self.m[2][0], self.m[3][0]],
                [self.m[0][1], self.m[1][1], self.m[2][1], self.m[3][1]],
                [self.m[0][2], self.m[1][2], self.m[2][2], self.m[3][2]],
                [self.m[0][3], self.m[1][3], self.m[2][3], self.m[3][3]]
            ]
        }
    }

    pub fn transform_point(&self, v: &Vector3f) -> Vector3f {
        let x = self.m[0][0] * v.x + self.m[0][1] * v.y + self.m[0][2] * v.z + self.m[0][3];
        let y = self.m[1][0] * v.x + self.m[1][1] * v.y + self.m[1][2] * v.z + self.m[1][3];
        let z = self.m[2][0] * v.x + self.m[2][1] * v.y + self.m[2][2] * v.z + self.m[2][3];
        let w = self.m[3][0] * v.x + self.m[3][1] * v.y + self.m[3][2] * v.z + self.m[3][3];
        let res = Vector3f::new(x, y, z);
        if w != 1.0 {
             res * (1.0/w)
        }
        else {
            res
        }
    }

    pub fn transform_direction(&self, v: &Vector3f) -> Vector3f {
        let x = self.m[0][0] * v.x + self.m[0][1] * v.y + self.m[0][2] * v.z;
        let y = self.m[1][0] * v.x + self.m[1][1] * v.y + self.m[1][2] * v.z;
        let z = self.m[2][0] * v.x + self.m[2][1] * v.y + self.m[2][2] * v.z;
        Vector3f::new(x, y, z)
    }

    pub fn transform_normal(&self, v: &Vector3f) -> Vector3f {
        let x = self.m[0][0] * v.x + self.m[1][0] * v.y + self.m[2][0] * v.z;
        let y = self.m[0][1] * v.x + self.m[1][1] * v.y + self.m[2][1] * v.z;
        let z = self.m[0][2] * v.x + self.m[1][2] * v.y + self.m[2][2] * v.z;
        Vector3f::new(x, y, z)
    }
}

impl Mul for &Matrix4 {
    type Output = Matrix4;

    fn mul(self, o: Self) -> Self::Output {
        let mut res = Matrix4::zero();
        let m = &mut res.m;
        for i in  0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    m[i][j] += self.m[i][k] * o.m[k][j];
                }
            }
        }
        
        res
    }

}

impl Mul for Matrix4 {
    type Output = Self;

    fn mul(self, o: Self) -> Self::Output {
        &self * &o
    }
}

impl Mul<&Vector3f> for &Matrix4 {
    type Output = Vector3f;

    fn mul(self, v: &Self::Output) -> Self::Output {
        let x = self.m[0][0] * v.x + self.m[0][1] * v.y + self.m[0][2] * v.z + self.m[0][3];
        let y = self.m[1][0] * v.x + self.m[1][1] * v.y + self.m[1][2] * v.z + self.m[1][3];
        let z = self.m[2][0] * v.x + self.m[2][1] * v.y + self.m[2][2] * v.z + self.m[2][3];
        let w = self.m[3][0] * v.x + self.m[3][1] * v.y + self.m[3][2] * v.z + self.m[3][3];
        let res = Vector3f::new(x, y, z);
        if w != 1.0 {
             res * (1.0/w)
        }
        else {
            res
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rotation_x() {
        let unit_x = Vector3f::new(1.0, 0.0, 0.0);
        let unit_y = Vector3f::new(0.0, 1.0, 0.0);
        let unit_z = Vector3f::new(0.0, 0.0, 1.0);
        
        let m = Matrix4::rotation_x(f64::consts::PI / 2.0);
        assert_eq!(unit_x, &m * &unit_x); // unit x stays the same
        assert!(unit_z.quite_same(&(&m * &unit_y))); // unit y maps to unit z
        assert!((unit_y * -1.0).quite_same(&(&m * &unit_z))); // unit z maps to opposite unit y

        assert!(true);
    }
}
