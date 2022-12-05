use crate::geom::intersectable::{Intersectable, Intersection};
use crate::geom::ray::Ray;
use crate::geom::vector3;
use crate::geom::vector3::Vector3f;

pub struct AABox {
  min: Vector3f,
  max: Vector3f
}

impl AABox {
    pub fn new(extend: &Vector3f) -> Self {
      let half_extend = extend * 0.5;
      Self {
        min: Vector3f::zero() - half_extend,
        max: half_extend
      }
    }
}

struct NormalAndDerivatives {
  normal: Vector3f,
  du: Vector3f,
  dv: Vector3f
}

impl AABox {
  const X_NEG:NormalAndDerivatives = NormalAndDerivatives {
    normal: Vector3f::new(-1.0, 0.0, 0.0),
    du: Vector3f::new(0.0, 0.0, -1.0),
    dv: Vector3f::new(0.0, 1.0, 0.0)
  };

  const X_POS:NormalAndDerivatives = NormalAndDerivatives {
    normal: Vector3f::new(1.0, 0.0, 0.0),
    du: Vector3f::new(0.0, 0.0, 1.0),
    dv: Vector3f::new(0.0, 1.0, 0.0)
  };

  const Y_NEG:NormalAndDerivatives = NormalAndDerivatives {
    normal: Vector3f::new(0.0, -1.0, 0.0),
    du: Vector3f::new(-1.0, 0.0, 0.0),
    dv: Vector3f::new(0.0, 0.0, 1.0)
  };

  const Y_POS:NormalAndDerivatives = NormalAndDerivatives {
    normal: Vector3f::new(0.0, 1.0, 0.0),
    du: Vector3f::new(1.0, 0.0, 0.0),
    dv: Vector3f::new(0.0, 0.0, 1.0)
  };

  const Z_NEG:NormalAndDerivatives = NormalAndDerivatives {
    normal: Vector3f::new(0.0, 0.0, -1.0),
    du: Vector3f::new(0.0, -1.0, 0.0),
    dv: Vector3f::new(1.0, 0.0, 0.0)
  };

  const Z_POS:NormalAndDerivatives = NormalAndDerivatives {
    normal: Vector3f::new(0.0, 0.0, 1.0),
    du: Vector3f::new(0.0, 1.0, 0.0),
    dv: Vector3f::new(1.0, 0.0, 0.0)
  };
}

impl Intersectable for AABox {
  // See https://www.scratchapixel.com/lessons/3d-basic-rendering/minimal-ray-tracer-rendering-simple-shapes/ray-AABox-intersection
  fn intersect(&self, ray: &Ray, near: f64, far: f64) -> Option<Intersection> {
    let ref ray_origin = ray.origin;
    let ref ray_direction = ray.direction;
    let inv_dir = Vector3f::new(1.0 / ray_direction.x, 1.0 / ray_direction.y, 1.0 / ray_direction.z);
    let mut tmin: f64;
    let mut tmax: f64;
    let mut normal_min: &NormalAndDerivatives;
    let mut normal_max: &NormalAndDerivatives;

    // println!("Origin : {:?} Direction: {:?}", &ray_origin, &ray_direction);
    // println!("MIN : {:?} MAX: {:?}", &self.min, &self.max);
  
    if (inv_dir.x >= 0.0) { 
        tmin = (self.min.x - ray_origin.x) * inv_dir.x; 
        tmax = (self.max.x - ray_origin.x) * inv_dir.x;
        normal_min = &AABox::X_NEG;
        normal_max = &AABox::X_POS;
    } 
    else { 
        tmin = (self.max.x - ray_origin.x) * inv_dir.x; 
        tmax = (self.min.x - ray_origin.x) * inv_dir.x; 
        normal_min = &AABox::X_POS;
        normal_max = &AABox::X_NEG;
    } 

    // println!("tmin {:?} | tmax {:?}", tmin, tmax);

    let tymin: f64;
    let tymax: f64;
    let normal_ymin: &NormalAndDerivatives;
    let normal_ymax: &NormalAndDerivatives;

    if inv_dir.y >= 0.0 { 
      tymin = (self.min.y - ray_origin.y) * inv_dir.y; 
      tymax = (self.max.y - ray_origin.y) * inv_dir.y; 
      normal_ymin = &AABox::Y_NEG;
      normal_ymax = &AABox::Y_POS;
    } 
    else { 
      tymin = (self.max.y - ray_origin.y) * inv_dir.y; 
      tymax = (self.min.y - ray_origin.y) * inv_dir.y; 
      normal_ymin = &AABox::Y_POS;
      normal_ymax = &AABox::Y_NEG;
    } 
 
    // println!("tymin {:?} | tymax {:?}", tymin, tymax);

    if (tmin > tymax) || (tymin > tmax)  {
      return None; 
    }
 
    if tymin > tmin {
      tmin = tymin; 
      normal_min = normal_ymin;
      // println!("Change for tymin");
    } 
 
    if tymax < tmax {
      tmax = tymax;
      normal_max = normal_ymax;
      // println!("Change for tymax");
    }
 
    let tzmin: f64;
    let tzmax: f64;
    let normal_zmin: &NormalAndDerivatives;
    let normal_zmax: &NormalAndDerivatives;

    if inv_dir.z >= 0.0 { 
      tzmin = (self.min.z - ray_origin.z) * inv_dir.z; 
      tzmax = (self.max.z - ray_origin.z) * inv_dir.z;
      normal_zmin = &AABox::Z_NEG;
      normal_zmax = &AABox::Z_POS;
    } 
    else { 
      tzmin = (self.max.z - ray_origin.z) * inv_dir.z; 
      tzmax = (self.min.z - ray_origin.z) * inv_dir.z; 
      normal_zmin = &AABox::Z_POS;
      normal_zmax = &AABox::Z_NEG;
    } 
 
    // println!("tzmin {:?} | tzmax {:?}", tzmin, tzmax);

    if (tmin > tzmax) || (tzmin > tmax) {
      return None;
    }
 
    if (tzmin > tmin) {
      tmin = tzmin;
      normal_min = normal_zmin;
      // println!("Change for tzmin");
    }
 
    if (tzmax < tmax) {
      tmax = tzmax; 
      normal_max = normal_zmax;
      // println!("Change for tzmax");
    }

    // println!("tmin {:?} | tmax {:?}", tmin, tmax);

    let mut t = tmin;
    let mut normal = normal_min;
    if t < 0.0 {
      t = tmax;
      normal = normal_max;
    };
    if t < 0.0 {
      return None;
    }

    //println!("Normal {:?}", &normal);

    let p = ray.origin + ray.direction * t;

    Some(Intersection {
        p,
        d: t,
        n: normal.normal,
        wo: &ray.direction * -1.0,
        u: vector3::dot(&p, &normal.du),
        v: vector3::dot(&p, &normal.dv),
        dpdu: normal.du,
        dpdv: normal.dv
    })
  }


  fn contain_point(&self, point: &Vector3f) -> bool {
    point.x >= self.min.x
    && point.x <= self.max.x
    && point.y >= self.min.y
    && point.y <= self.max.y
    && point.z >= self.min.z
    && point.z <= self.max.z
  }
}