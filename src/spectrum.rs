use std::ops::{Add, AddAssign, Div, Mul};

#[derive(Clone, Copy, Debug)]
pub struct Spectrum {
    spectrum: [f64; 3],
}

impl Spectrum {
    pub const fn new(r: f64, g: f64, b: f64) -> Self {
        Self { spectrum: [r, g, b] }
    }

    pub fn to_rgb(&self) -> Vec<u8> {
        let mut res = vec![0, 0, 0, 255];
        res[0] = (in_bound(self.spectrum[0]) * 255.0) as u8;
        res[1] = (in_bound(self.spectrum[1]) * 255.0) as u8;
        res[2] = (in_bound(self.spectrum[2]) * 255.0) as u8;
        res
    }

    pub fn gamma_correct(&mut self) {
        self.spectrum[0] = self.spectrum[0].sqrt();
        self.spectrum[1] = self.spectrum[1].sqrt();
        self.spectrum[2] = self.spectrum[2].sqrt();
    }
}

impl Add for Spectrum {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        Spectrum::new(
            self.spectrum[0] + other.spectrum[0],
            self.spectrum[1] + other.spectrum[1],
            self.spectrum[2] + other.spectrum[2],
        )
    }
}

impl AddAssign for Spectrum {
    fn add_assign(&mut self, other: Self) {
        self.spectrum[0] += other.spectrum[0];
        self.spectrum[1] += other.spectrum[1];
        self.spectrum[2] += other.spectrum[2];
    }
}

// TODO Avoid copy/pasted code

impl Mul<&Spectrum> for Spectrum {
    type Output = Self;

    fn mul(self, other: &Self) -> Self::Output {
        Spectrum::new(
            self.spectrum[0] * other.spectrum[0],
            self.spectrum[1] * other.spectrum[1],
            self.spectrum[2] * other.spectrum[2],
        )
    }
}

impl Mul<&Spectrum> for &Spectrum {
    type Output = Spectrum;

    fn mul(self, other: &Spectrum) -> Spectrum {
        Spectrum::new(
            self.spectrum[0] * other.spectrum[0],
            self.spectrum[1] * other.spectrum[1],
            self.spectrum[2] * other.spectrum[2],
        )
    }
}

impl Mul<f64> for &Spectrum {
    type Output = Spectrum;

    fn mul(self, scale: f64) -> Self::Output {
        Spectrum::new(self.spectrum[0] * scale, self.spectrum[1] * scale, self.spectrum[2] * scale)
    }
}

impl Mul<f64> for Spectrum {
    type Output = Spectrum;

    fn mul(self, scale: f64) -> Self::Output {
        Spectrum::new(self.spectrum[0] * scale, self.spectrum[1] * scale, self.spectrum[2] * scale)
    }
}

impl Div<f64> for Spectrum {
    type Output = Spectrum;

    fn div(self, scale: f64) -> Self::Output {
        Spectrum::new(self.spectrum[0] / scale, self.spectrum[1] / scale, self.spectrum[2] / scale)
    }
}

fn in_bound(v: f64) -> f64 {
    f64::min(1.0, f64::max(0.0, v))
}
