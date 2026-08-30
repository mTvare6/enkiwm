// SPDX-License-Identifier: MPL-2.0

use smithay::utils::{Point, Size};
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

use core::fmt;
use core::ops::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct IVec2 {
    pub x: i32,
    pub y: i32,
}

impl IVec2 {
    pub const ZERO: Self = Self::splat(0);
    pub const ONE: Self = Self::splat(1);
    pub const NEG_ONE: Self = Self::splat(-1);
    pub const FLIP_X: Self = Self::new(-1, 1);
    pub const FLIP_Y: Self = Self::new(1, -1);
    pub const X: Self = Self::new(1, 0);
    pub const Y: Self = Self::new(0, 1);
    pub const NEG_X: Self = Self::new(-1, 0);
    pub const NEG_Y: Self = Self::new(0, -1);
    pub const ALIGNED_AXES: [Self; 2] = [Self::X, Self::Y];
    pub const AXES: [Self; 4] = [
        Self::X,
        Self::Y,
        Self::NEG_X,
        Self::NEG_Y,
    ];

    #[inline(always)]
    pub const fn new(x: i32, y: i32) -> Self { Self { x, y } }

    #[inline]
    pub const fn splat(v: i32) -> Self { Self { x: v, y: v } }

    #[inline]
    pub fn map<F>(self, f: F) -> Self
    where
        F: Fn(i32) -> i32,
    {
        Self::new(f(self.x), f(self.y))
    }

    #[inline]
    pub fn dot(self, rhs: Self) -> i32 { (self.x * rhs.x) + (self.y * rhs.y) }

    #[inline]
    pub fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    #[inline]
    pub fn signum(self) -> Self {
        Self {
            x: self.x.signum(),
            y: self.y.signum(),
        }
    }

    #[inline]
    pub fn to_tuple(self) -> (i32, i32) { (self.x, self.y) }

    #[inline]
    pub fn from_tuple(val: (i32, i32)) -> Self {
        Self {
            x: val.0,
            y: val.1,
        }
    }

    #[inline]
    pub fn from_size<K>(val: Size<i32, K>) -> Self {
        let val: (i32, i32) = val.into();
        Self {
            x: val.0,
            y: val.1,
        }
    }

    #[inline]
    pub fn to_size<K>(self) -> Size<i32, K> { self.to_tuple().into() }

    #[inline]
    pub fn from_point<K>(val: Point<i32, K>) -> Self {
        let val: (i32, i32) = val.into();
        Self {
            x: val.0,
            y: val.1,
        }
    }

    #[inline]
    pub fn to_point<K>(self) -> Point<i32, K> { self.to_tuple().into() }

    #[inline]
    pub fn length_squared(self) -> i32 { self.dot(self) }

    #[inline]
    pub fn distance_squared(self, rhs: Self) -> i32 { (self - rhs).length_squared() }

    #[inline]
    pub fn manhattan_distance(self, rhs: Self) -> u32 { self.x.abs_diff(rhs.x) + self.y.abs_diff(rhs.y) }

    #[inline]
    pub fn chebyshev_distance(self, rhs: Self) -> u32 { self.x.abs_diff(rhs.x).max(self.y.abs_diff(rhs.y)) }

    #[inline]
    pub fn perp(self) -> Self {
        Self {
            x: -self.y,
            y: self.x,
        }
    }

    #[inline]
    pub fn det(self, rhs: Self) -> i32 { (self.x * rhs.y) - (self.y * rhs.x) }

    #[inline]
    pub fn rotate(self, rhs: Self) -> Self {
        Self {
            x: self.x * rhs.x - self.y * rhs.y,
            y: self.y * rhs.x + self.x * rhs.y,
        }
    }
}

impl Default for IVec2 {
    #[inline(always)]
    fn default() -> Self { Self::ZERO }
}

impl Div for IVec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self {
        Self {
            x: self.x.div(rhs.x),
            y: self.y.div(rhs.y),
        }
    }
}

impl Div<&Self> for IVec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: &Self) -> Self { self.div(*rhs) }
}

impl Div<&IVec2> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn div(self, rhs: &IVec2) -> IVec2 { (*self).div(*rhs) }
}

impl Div<IVec2> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn div(self, rhs: IVec2) -> IVec2 { (*self).div(rhs) }
}

impl DivAssign for IVec2 {
    #[inline]
    fn div_assign(&mut self, rhs: Self) {
        self.x.div_assign(rhs.x);
        self.y.div_assign(rhs.y);
    }
}

impl DivAssign<&Self> for IVec2 {
    #[inline]
    fn div_assign(&mut self, rhs: &Self) { self.div_assign(*rhs); }
}

impl Div<i32> for IVec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: i32) -> Self {
        Self {
            x: self.x.div(rhs),
            y: self.y.div(rhs),
        }
    }
}

impl Div<&i32> for IVec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: &i32) -> Self { self.div(*rhs) }
}

impl Div<&i32> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn div(self, rhs: &i32) -> IVec2 { (*self).div(*rhs) }
}

impl Div<i32> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn div(self, rhs: i32) -> IVec2 { (*self).div(rhs) }
}

impl DivAssign<i32> for IVec2 {
    #[inline]
    fn div_assign(&mut self, rhs: i32) {
        self.x.div_assign(rhs);
        self.y.div_assign(rhs);
    }
}

impl DivAssign<&i32> for IVec2 {
    #[inline]
    fn div_assign(&mut self, rhs: &i32) { self.div_assign(*rhs); }
}

impl Div<IVec2> for i32 {
    type Output = IVec2;
    #[inline]
    fn div(self, rhs: IVec2) -> IVec2 {
        IVec2 {
            x: self.div(rhs.x),
            y: self.div(rhs.y),
        }
    }
}

impl Div<&IVec2> for i32 {
    type Output = IVec2;
    #[inline]
    fn div(self, rhs: &IVec2) -> IVec2 { self.div(*rhs) }
}

impl Div<&IVec2> for &i32 {
    type Output = IVec2;
    #[inline]
    fn div(self, rhs: &IVec2) -> IVec2 { (*self).div(*rhs) }
}

impl Div<IVec2> for &i32 {
    type Output = IVec2;
    #[inline]
    fn div(self, rhs: IVec2) -> IVec2 { (*self).div(rhs) }
}

impl Mul for IVec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.x.mul(rhs.x),
            y: self.y.mul(rhs.y),
        }
    }
}

impl Mul<&Self> for IVec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: &Self) -> Self { self.mul(*rhs) }
}

impl Mul<&IVec2> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn mul(self, rhs: &IVec2) -> IVec2 { (*self).mul(*rhs) }
}

impl Mul<IVec2> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn mul(self, rhs: IVec2) -> IVec2 { (*self).mul(rhs) }
}

impl MulAssign for IVec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        self.x.mul_assign(rhs.x);
        self.y.mul_assign(rhs.y);
    }
}

impl MulAssign<&Self> for IVec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: &Self) { self.mul_assign(*rhs); }
}

impl Mul<i32> for IVec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: i32) -> Self {
        Self {
            x: self.x.mul(rhs),
            y: self.y.mul(rhs),
        }
    }
}

impl Mul<&i32> for IVec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: &i32) -> Self { self.mul(*rhs) }
}

impl Mul<&i32> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn mul(self, rhs: &i32) -> IVec2 { (*self).mul(*rhs) }
}

impl Mul<i32> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn mul(self, rhs: i32) -> IVec2 { (*self).mul(rhs) }
}

impl MulAssign<i32> for IVec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: i32) {
        self.x.mul_assign(rhs);
        self.y.mul_assign(rhs);
    }
}

impl MulAssign<&i32> for IVec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: &i32) { self.mul_assign(*rhs); }
}

impl Mul<IVec2> for i32 {
    type Output = IVec2;
    #[inline]
    fn mul(self, rhs: IVec2) -> IVec2 {
        IVec2 {
            x: self.mul(rhs.x),
            y: self.mul(rhs.y),
        }
    }
}

impl Mul<&IVec2> for i32 {
    type Output = IVec2;
    #[inline]
    fn mul(self, rhs: &IVec2) -> IVec2 { self.mul(*rhs) }
}

impl Mul<&IVec2> for &i32 {
    type Output = IVec2;
    #[inline]
    fn mul(self, rhs: &IVec2) -> IVec2 { (*self).mul(*rhs) }
}

impl Mul<IVec2> for &i32 {
    type Output = IVec2;
    #[inline]
    fn mul(self, rhs: IVec2) -> IVec2 { (*self).mul(rhs) }
}

impl Add for IVec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x.add(rhs.x),
            y: self.y.add(rhs.y),
        }
    }
}

impl Add<&Self> for IVec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: &Self) -> Self { self.add(*rhs) }
}

impl Add<&IVec2> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn add(self, rhs: &IVec2) -> IVec2 { (*self).add(*rhs) }
}

impl Add<IVec2> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn add(self, rhs: IVec2) -> IVec2 { (*self).add(rhs) }
}

impl AddAssign for IVec2 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x.add_assign(rhs.x);
        self.y.add_assign(rhs.y);
    }
}

impl AddAssign<&Self> for IVec2 {
    #[inline]
    fn add_assign(&mut self, rhs: &Self) { self.add_assign(*rhs); }
}

impl Add<i32> for IVec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: i32) -> Self {
        Self {
            x: self.x.add(rhs),
            y: self.y.add(rhs),
        }
    }
}

impl Add<&i32> for IVec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: &i32) -> Self { self.add(*rhs) }
}

impl Add<&i32> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn add(self, rhs: &i32) -> IVec2 { (*self).add(*rhs) }
}

impl Add<i32> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn add(self, rhs: i32) -> IVec2 { (*self).add(rhs) }
}

impl AddAssign<i32> for IVec2 {
    #[inline]
    fn add_assign(&mut self, rhs: i32) {
        self.x.add_assign(rhs);
        self.y.add_assign(rhs);
    }
}

impl AddAssign<&i32> for IVec2 {
    #[inline]
    fn add_assign(&mut self, rhs: &i32) { self.add_assign(*rhs); }
}

impl Add<IVec2> for i32 {
    type Output = IVec2;
    #[inline]
    fn add(self, rhs: IVec2) -> IVec2 {
        IVec2 {
            x: self.add(rhs.x),
            y: self.add(rhs.y),
        }
    }
}

impl Add<&IVec2> for i32 {
    type Output = IVec2;
    #[inline]
    fn add(self, rhs: &IVec2) -> IVec2 { self.add(*rhs) }
}

impl Add<&IVec2> for &i32 {
    type Output = IVec2;
    #[inline]
    fn add(self, rhs: &IVec2) -> IVec2 { (*self).add(*rhs) }
}

impl Add<IVec2> for &i32 {
    type Output = IVec2;
    #[inline]
    fn add(self, rhs: IVec2) -> IVec2 { (*self).add(rhs) }
}

impl Sub for IVec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x.sub(rhs.x),
            y: self.y.sub(rhs.y),
        }
    }
}

impl Sub<&Self> for IVec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: &Self) -> Self { self.sub(*rhs) }
}

impl Sub<&IVec2> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn sub(self, rhs: &IVec2) -> IVec2 { (*self).sub(*rhs) }
}

impl Sub<IVec2> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn sub(self, rhs: IVec2) -> IVec2 { (*self).sub(rhs) }
}

impl SubAssign for IVec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.x.sub_assign(rhs.x);
        self.y.sub_assign(rhs.y);
    }
}

impl SubAssign<&Self> for IVec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: &Self) { self.sub_assign(*rhs); }
}

impl Sub<i32> for IVec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: i32) -> Self {
        Self {
            x: self.x.sub(rhs),
            y: self.y.sub(rhs),
        }
    }
}

impl Sub<&i32> for IVec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: &i32) -> Self { self.sub(*rhs) }
}

impl Sub<&i32> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn sub(self, rhs: &i32) -> IVec2 { (*self).sub(*rhs) }
}

impl Sub<i32> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn sub(self, rhs: i32) -> IVec2 { (*self).sub(rhs) }
}

impl SubAssign<i32> for IVec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: i32) {
        self.x.sub_assign(rhs);
        self.y.sub_assign(rhs);
    }
}

impl SubAssign<&i32> for IVec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: &i32) { self.sub_assign(*rhs); }
}

impl Sub<IVec2> for i32 {
    type Output = IVec2;
    #[inline]
    fn sub(self, rhs: IVec2) -> IVec2 {
        IVec2 {
            x: self.sub(rhs.x),
            y: self.sub(rhs.y),
        }
    }
}

impl Sub<&IVec2> for i32 {
    type Output = IVec2;
    #[inline]
    fn sub(self, rhs: &IVec2) -> IVec2 { self.sub(*rhs) }
}

impl Sub<&IVec2> for &i32 {
    type Output = IVec2;
    #[inline]
    fn sub(self, rhs: &IVec2) -> IVec2 { (*self).sub(*rhs) }
}

impl Sub<IVec2> for &i32 {
    type Output = IVec2;
    #[inline]
    fn sub(self, rhs: IVec2) -> IVec2 { (*self).sub(rhs) }
}

impl Rem for IVec2 {
    type Output = Self;
    #[inline]
    fn rem(self, rhs: Self) -> Self {
        Self {
            x: self.x.rem(rhs.x),
            y: self.y.rem(rhs.y),
        }
    }
}

impl Rem<&Self> for IVec2 {
    type Output = Self;
    #[inline]
    fn rem(self, rhs: &Self) -> Self { self.rem(*rhs) }
}

impl Rem<&IVec2> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn rem(self, rhs: &IVec2) -> IVec2 { (*self).rem(*rhs) }
}

impl Rem<IVec2> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn rem(self, rhs: IVec2) -> IVec2 { (*self).rem(rhs) }
}

impl RemAssign for IVec2 {
    #[inline]
    fn rem_assign(&mut self, rhs: Self) {
        self.x.rem_assign(rhs.x);
        self.y.rem_assign(rhs.y);
    }
}

impl RemAssign<&Self> for IVec2 {
    #[inline]
    fn rem_assign(&mut self, rhs: &Self) { self.rem_assign(*rhs); }
}

impl Rem<i32> for IVec2 {
    type Output = Self;
    #[inline]
    fn rem(self, rhs: i32) -> Self {
        Self {
            x: self.x.rem(rhs),
            y: self.y.rem(rhs),
        }
    }
}

impl Rem<&i32> for IVec2 {
    type Output = Self;
    #[inline]
    fn rem(self, rhs: &i32) -> Self { self.rem(*rhs) }
}

impl Rem<&i32> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn rem(self, rhs: &i32) -> IVec2 { (*self).rem(*rhs) }
}

impl Rem<i32> for &IVec2 {
    type Output = IVec2;
    #[inline]
    fn rem(self, rhs: i32) -> IVec2 { (*self).rem(rhs) }
}

impl RemAssign<i32> for IVec2 {
    #[inline]
    fn rem_assign(&mut self, rhs: i32) {
        self.x.rem_assign(rhs);
        self.y.rem_assign(rhs);
    }
}

impl RemAssign<&i32> for IVec2 {
    #[inline]
    fn rem_assign(&mut self, rhs: &i32) { self.rem_assign(*rhs); }
}

impl Rem<IVec2> for i32 {
    type Output = IVec2;
    #[inline]
    fn rem(self, rhs: IVec2) -> IVec2 {
        IVec2 {
            x: self.rem(rhs.x),
            y: self.rem(rhs.y),
        }
    }
}

impl Rem<&IVec2> for i32 {
    type Output = IVec2;
    #[inline]
    fn rem(self, rhs: &IVec2) -> IVec2 { self.rem(*rhs) }
}

impl Rem<&IVec2> for &i32 {
    type Output = IVec2;
    #[inline]
    fn rem(self, rhs: &IVec2) -> IVec2 { (*self).rem(*rhs) }
}

impl Rem<IVec2> for &i32 {
    type Output = IVec2;
    #[inline]
    fn rem(self, rhs: IVec2) -> IVec2 { (*self).rem(rhs) }
}

impl AsRef<[i32; 2]> for IVec2 {
    #[inline]
    fn as_ref(&self) -> &[i32; 2] { unsafe { &*(self as *const Self as *const [i32; 2]) } }
}

impl AsMut<[i32; 2]> for IVec2 {
    #[inline]
    fn as_mut(&mut self) -> &mut [i32; 2] { unsafe { &mut *(self as *mut Self as *mut [i32; 2]) } }
}

impl fmt::Display for IVec2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "[{}, {}]", self.x, self.y) }
}

impl fmt::Debug for IVec2 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result { fmt.debug_tuple(stringify!(IVec2)).field(&self.x).field(&self.y).finish() }
}

impl From<[i32; 2]> for IVec2 {
    #[inline]
    fn from(a: [i32; 2]) -> Self { Self::new(a[0], a[1]) }
}

impl From<IVec2> for [i32; 2] {
    #[inline]
    fn from(v: IVec2) -> Self { [v.x, v.y] }
}

impl From<(i32, i32)> for IVec2 {
    #[inline]
    fn from(t: (i32, i32)) -> Self { Self::new(t.0, t.1) }
}

impl From<IVec2> for (i32, i32) {
    #[inline]
    fn from(v: IVec2) -> Self { (v.x, v.y) }
}

impl<K> From<IVec2> for Point<i32, K> {
    fn from(val: IVec2) -> Self {
        let t = val.to_tuple();
        t.into()
    }
}

impl<K> From<Point<i32, K>> for IVec2 {
    fn from(val: Point<i32, K>) -> Self { Self::new(val.x, val.y) }
}

impl<K> From<IVec2> for Size<i32, K> {
    fn from(val: IVec2) -> Self {
        let t = val.to_tuple();
        t.into()
    }
}

impl<K> From<Size<i32, K>> for IVec2 {
    fn from(size: Size<i32, K>) -> Self { IVec2::new(size.w, size.h) }
}
