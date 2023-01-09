use std::ops::*;

pub type Vector2 = Vector<2>;
pub type Vector3 = Vector<3>;

#[derive(Copy, Clone, Debug)]
pub struct Vector<const DIMENSIONS: usize> {
    data: [f32; DIMENSIONS],
}

impl<const DIMENSIONS: usize> Vector<DIMENSIONS> {
    pub fn zero() -> Self {
        Self {
            data: [0f32; DIMENSIONS],
        }
    }

    pub fn distance(self, other: Self) -> f32 {
        (0..DIMENSIONS)
            .map(|i| other.data[i] - self.data[i])
            .sum::<f32>()
            .sqrt()
    }

    pub fn normalize(self) -> Self {
        let inv_len = 1.0 / Self::distance(self, Vector::zero());
        self * inv_len
        // let d = Self::distance(self, Vec3::ZERO);
        // self * d
    }
}

impl<const DIMENSIONS: usize> Add for Vector<DIMENSIONS> {
    type Output = Vector<DIMENSIONS>;
    fn add(self, rhs: Self) -> Self::Output {
        let mut out = Vector::zero();
        (0..DIMENSIONS).for_each(|i| out.data[i] = self.data[i] + rhs.data[i]);
        out
    }
}

impl<const DIMENSIONS: usize> AddAssign for Vector<DIMENSIONS> {
    fn add_assign(&mut self, rhs: Self) {
        (0..DIMENSIONS).for_each(|i| self.data[i] += rhs.data[i]);
    }
}

impl<const DIMENSIONS: usize> Mul<f32> for Vector<DIMENSIONS> {
    type Output = Vector<DIMENSIONS>;

    fn mul(self, rhs: f32) -> Self::Output {
        let mut out = Vector::zero();
        (0..DIMENSIONS).for_each(|i| out.data[i] = self.data[i] * rhs);
        out
    }
}

impl<const DIMENSIONS: usize> Sub for Vector<DIMENSIONS> {
    type Output = Vector<DIMENSIONS>;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut out = Vector::zero();
        (0..DIMENSIONS).for_each(|i| out.data[i] = self.data[i] - rhs.data[i]);
        out
    }
}

// impl Vector for Vec3 {
//     fn distance(self, other: Self) -> f32 {
//         ((other.x - self.x).powi(2) + (other.y - self.y).powi(2) + (other.z - self.z).powi(2))
//             .sqrt()
//     }

//     fn normalize(self) -> Self {
//         let d = Self::distance(self, Vec3::ZERO);
//         self * d
//     }
// }

// pub trait Vector:
//     Sized
//     + Copy
//     + Clone
//     + Sub<Output = Self>
//     + Sized
//     + Mul<f32, Output = Self>
//     + Add<Output = Self>
//     + AddAssign
// {
//     fn distance(self, other: Self) -> f32;
//     fn normalize(self) -> Self;
// }

// #[derive(Copy, Clone, Debug, PartialEq)]
// pub struct Vec3 {
//     pub x: f32,
//     pub y: f32,
//     pub z: f32,
// }

// pub fn vec3(x: f32, y: f32, z: f32) -> Vec3 {
//     Vec3 { x, y, z }
// }

// impl Vec3 {
//     const ZERO: Vec3 = Vec3 {
//         x: 0.,
//         y: 0.,
//         z: 0.,
//     };
// }

// impl Add for Vec3 {
//     type Output = Vec3;
//     fn add(self, rhs: Self) -> Self::Output {
//         vec3(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
//     }
// }

// impl AddAssign for Vec3 {
//     fn add_assign(&mut self, rhs: Self) {
//         self.x += rhs.x;
//         self.y += rhs.y;
//         self.z += rhs.z;
//     }
// }

// // impl Mul for Vec3 {
// //     type Output = Vec3;

// //     fn mul(self, rhs: Self) -> Self::Output {
// //         vec3(self.x * rhs.x, self.y * rhs.y, self.z * rhs.z)
// //     }
// // }

// impl Mul<f32> for Vec3 {
//     type Output = Vec3;

//     fn mul(self, rhs: f32) -> Self::Output {
//         vec3(self.x * rhs, self.y * rhs, self.z * rhs)
//     }
// }

// impl Sub for Vec3 {
//     type Output = Vec3;

//     fn sub(self, rhs: Self) -> Self::Output {
//         vec3(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
//     }
// }

// impl Vector for Vec3 {
//     fn distance(self, other: Self) -> f32 {
//         ((other.x - self.x).powi(2) + (other.y - self.y).powi(2) + (other.z - self.z).powi(2))
//             .sqrt()
//     }

//     fn normalize(self) -> Self {
//         let d = Self::distance(self, Vec3::ZERO);
//         self * d
//     }
// }

// pub struct Vec2 {
//     pub x: f32,
//     pub y: f32,
// }

// impl<T> Vector for T where T: Sub + Sized + Mul<f32, Output = T> + Add {}
