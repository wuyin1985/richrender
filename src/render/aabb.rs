use std::ops::Mul;
use std::cmp::Ordering;
use glam::*;
use glam::Vec3;

pub fn min<S: PartialOrd>(v1: S, v2: S) -> S {
    match v1.partial_cmp(&v2) {
        Some(Ordering::Less) => v1,
        _ => v2,
    }
}

/// Returns the max value of two PartialOrd values.
pub fn max<S: PartialOrd>(v1: S, v2: S) -> S {
    match v1.partial_cmp(&v2) {
        Some(Ordering::Greater) => v1,
        _ => v2,
    }
}

/// Return the partial minimum from an Iterator of PartialOrd if it exists.
pub fn partial_min<I, S>(iter: I) -> Option<S>
    where
        S: PartialOrd,
        I: Iterator<Item=S>,
{
    iter.min_by(|v1, v2| v1.partial_cmp(v2).unwrap_or(Ordering::Equal))
}

/// Return the partial maximum from an Iterator of PartialOrd if it exists.
pub fn partial_max<I, S>(iter: I) -> Option<S>
    where
        S: PartialOrd,
        I: Iterator<Item=S>,
{
    iter.max_by(|v1, v2| v1.partial_cmp(v2).unwrap_or(Ordering::Equal))
}


/// Axis aligned bounding box.
#[derive(Copy, Clone, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    /// Create a new AABB.
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Aabb { min, max }
    }
}

impl Aabb {
    /// Compute the union of several AABBs.
    pub fn union(aabbs: &[Aabb]) -> Option<Self> {
        if aabbs.is_empty() {
            None
        } else if aabbs.len() == 1 {
            Some(aabbs[0])
        } else {
            let min_x = partial_min(aabbs.iter().map(|aabb| aabb.min.x)).unwrap();
            let min_y = partial_min(aabbs.iter().map(|aabb| aabb.min.y)).unwrap();
            let min_z = partial_min(aabbs.iter().map(|aabb| aabb.min.z)).unwrap();
            let min = Vec3::new(min_x, min_y, min_z);

            let max_x = partial_max(aabbs.iter().map(|aabb| aabb.max.x)).unwrap();
            let max_y = partial_max(aabbs.iter().map(|aabb| aabb.max.y)).unwrap();
            let max_z = partial_max(aabbs.iter().map(|aabb| aabb.max.z)).unwrap();
            let max = Vec3::new(max_x, max_y, max_z);

            Some(Aabb::new(min, max))
        }
    }

    /// Get the size of the larger side of the AABB.
    pub fn get_larger_side_size(&self) -> f32 {
        let size = self.max - self.min;
        let x = size.x.abs();
        let y = size.y.abs();
        let z = size.z.abs();

        if x > y && x > z {
            x
        } else if y > z {
            y
        } else {
            z
        }
    }

    /// Get the center of the AABB.
    pub fn get_center(&self) -> Vec3 {
        let two = Vec3::new(2f32, 2f32, 2f32);
        self.min + (self.max - self.min) / two
    }
}


/// Scale the AABB by multiplying it by a BaseFloat
impl Mul<f32> for Aabb {
    type Output = Aabb;

    fn mul(self, rhs: f32) -> Self::Output {
        Aabb::new(self.min.mul(rhs), self.max.mul(rhs))
    }
}

impl Mul<Mat4> for Aabb {
    type Output = Aabb;

    fn mul(self, rhs: Mat4) -> Self::Output {
        let min = self.min;
        let min = rhs * Vec4::new(min.x, min.y, min.z, 1f32);

        let max = self.max;
        let max = rhs * Vec4::new(max.x, max.y, max.z, 1f32);

        let min_x = min.x.min(max.x);
        let min_y = min.y.min(max.y);
        let min_z = min.z.min(max.z);

        let max_x = min.x.max(max.x);
        let max_y = min.y.max(max.y);
        let max_z = min.z.max(max.z);

        Aabb::new(Vec3::new(min_x, min_y, min_z), Vec3::new(max_x, max_y, max_z))
    }
}
