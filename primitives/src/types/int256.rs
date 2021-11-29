// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::types::uint256::u256;
#[allow(unused_imports)]
use core::{
    fmt,
    ops::{
        Add, AddAssign, Deref, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub,
        SubAssign,
    },
    str::FromStr,
};
use liquid_prelude::vec::{from_elem, Vec};
use num::{
    bigint::{BigInt, ParseBigIntError},
    pow,
    traits::{
        ops::checked::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub},
        Signed,
    },
    Bounded, Num,
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Num, One, Zero)]
#[allow(non_camel_case_types)]
pub struct i256(pub BigInt);

impl i256 {
    /// Checked conversion to u256
    pub fn to_uint256(&self) -> Option<u256> {
        self.0
            .to_biguint()
            .map(u256)
            .filter(|value| *value <= u256::max_value() && *value >= u256::min_value())
    }

    pub fn to_be_bytes(&self) -> [u8; 32] {
        let bytes = self.0.to_signed_bytes_be();
        let mut res = [0u8; 32];
        if self.0 < BigInt::from(0) {
            res = [0xff; 32]
        }
        res[32 - bytes.len()..].copy_from_slice(&bytes);
        res
    }

    pub fn from_signed_le_bytes(bytes: &[u8]) -> Self {
        let value = num_bigint::BigInt::from_signed_bytes_le(bytes);
        Self(value)
    }

    pub fn from_signed_be_bytes(bytes: &[u8]) -> Self {
        let value = num_bigint::BigInt::from_signed_bytes_be(bytes);
        Self(value)
    }
}

impl Bounded for i256 {
    fn min_value() -> Self {
        lazy_static! {
            static ref MIN_VALUE: BigInt = -pow(BigInt::from(2), 255);
        }
        // -2**255
        i256(MIN_VALUE.clone())
    }
    fn max_value() -> Self {
        lazy_static! {
            static ref MAX_VALUE: BigInt = pow(BigInt::from(2), 255) - BigInt::from(1);
        }
        i256(MAX_VALUE.clone())
    }
}

impl Deref for i256 {
    type Target = BigInt;

    fn deref(&self) -> &BigInt {
        &self.0
    }
}

macro_rules! impl_from_int {
    ($T:ty) => {
        impl From<$T> for i256 {
            #[inline]
            fn from(n: $T) -> Self {
                i256(BigInt::from(n))
            }
        }
    };
}

impl_from_int!(i8);
impl_from_int!(i16);
impl_from_int!(i32);
impl_from_int!(i64);
impl_from_int!(i128);
impl_from_int!(isize);
impl_from_int!(u8);
impl_from_int!(u16);
impl_from_int!(u32);
impl_from_int!(u64);
impl_from_int!(u128);
impl_from_int!(usize);

impl<'a> From<&'a i256> for i256 {
    fn from(n: &i256) -> Self {
        n.clone()
    }
}

impl FromStr for i256 {
    type Err = ParseBigIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        if s.starts_with("0x") || s.starts_with("-0x") {
            if let Some(sub_str) = s.strip_prefix("0x") {
                Ok(BigInt::from_str_radix(sub_str, 16).map(Self)?)
            } else {
                Ok(-BigInt::from_str_radix(&s[3..], 16).map(Self)?)
            }
        } else {
            Ok(BigInt::from_str_radix(&s, 10).map(Self)?)
        }
    }
}

#[cfg(feature = "std")]
impl fmt::Display for i256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.to_str_radix(10))
    }
}

#[cfg(feature = "std")]
impl fmt::Debug for i256 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "i256({})", &self.0.to_str_radix(10))
    }
}

impl Signed for i256 {
    fn abs(&self) -> Self {
        i256(self.0.abs())
    }
    fn abs_sub(&self, other: &Self) -> Self {
        i256(self.0.abs_sub(&other.0))
    }
    fn signum(&self) -> Self {
        i256(self.0.signum())
    }
    fn is_positive(&self) -> bool {
        self.0.is_positive()
    }
    fn is_negative(&self) -> bool {
        self.0.is_negative()
    }
}

/// A macro that forwards an unary operator trait i.e. Add
macro_rules! forward_op {
    (impl $trait_:ident for $type_:ident { fn $method:ident }) => {
        impl $trait_<$type_> for $type_ {
            type Output = $type_;

            fn $method(self, $type_(b): $type_) -> $type_ {
                let $type_(a) = self;
                let res = $type_(a.$method(&b));
                if res > i256::max_value() {
                    panic!("attempt to {} with overflow", stringify!($method));
                } else if res < i256::min_value() {
                    panic!("attempt to {} with underflow", stringify!($method));
                } else {
                    res
                }
            }
        }
    };
}

/// A macro that forwards a checked operator i.e. CheckedAdd
macro_rules! forward_checked_op {
    (impl $trait_:ident for $type_:ident { fn $method:ident }) => {
        impl $trait_ for $type_ {
            fn $method(&self, $type_(b): &$type_) -> Option<$type_> {
                let $type_(a) = self;
                a.$method(&b)
                    .filter(|value| {
                        value >= &i256::min_value() && value <= &i256::max_value()
                    })
                    .map($type_)
            }
        }
    };
}

/// A macro that forwards a assignment operator i.e. AddAssign
macro_rules! forward_assign_op {
    (impl $trait_:ident for $type_:ident { fn $method:ident }) => {
        impl $trait_ for $type_ {
            fn $method(&mut self, $type_(b): $type_) {
                // Borrow happens only inside this scope
                {
                    let a = &mut self.0;
                    a.$method(b);
                }
                // Check bounds
                if *self > i256::max_value() {
                    panic!("attempt to {} with overflow", stringify!($method));
                }
                if *self < i256::min_value() {
                    panic!("attempt to {} with underflow", stringify!($method));
                }
            }
        }
    };
}

macro_rules! forward_unary_op {
    (impl $trait_:ident for $type_:ident { fn $method:ident }) => {
        impl $trait_ for $type_ {
            type Output = $type_;

            fn $method(self) -> $type_ {
                let $type_(a) = self;
                let res = $type_(a.$method());
                // Check bounds
                if res > i256::max_value() {
                    panic!("attempt to {} with overflow", stringify!($method));
                }
                if res < i256::min_value() {
                    panic!("attempt to {} with underflow", stringify!($method));
                }

                res
            }
        }
    };
}

forward_op! { impl Add for i256 { fn add } }
forward_checked_op! { impl CheckedAdd for i256 { fn checked_add } }
forward_assign_op! { impl AddAssign for i256 { fn add_assign } }

forward_op! { impl Sub for i256 { fn sub } }
forward_checked_op! { impl CheckedSub for i256 { fn checked_sub } }
forward_assign_op! { impl SubAssign for i256 { fn sub_assign } }

forward_op! { impl Mul for i256 { fn mul } }
forward_checked_op! { impl CheckedMul for i256 { fn checked_mul } }
forward_assign_op! { impl MulAssign for i256 { fn mul_assign } }

forward_op! { impl Div for i256 { fn div } }
forward_checked_op! { impl CheckedDiv for i256 { fn checked_div } }
forward_assign_op! { impl DivAssign for i256 { fn div_assign } }

forward_op! { impl Rem for i256 { fn rem } }
forward_assign_op! { impl RemAssign for i256 { fn rem_assign } }

forward_unary_op! { impl Neg for i256 { fn neg } }

impl scale::Encode for i256 {
    fn size_hint(&self) -> usize {
        32
    }
    fn encode(&self) -> Vec<u8> {
        let encoded_data = self.to_be_bytes();
        let size = encoded_data.len();
        let mut buf = Vec::with_capacity(size);
        buf.extend(encoded_data);
        buf
    }
}

impl scale::Decode for i256 {
    fn decode<I: scale::Input>(value: &mut I) -> Result<Self, scale::Error> {
        let size = 32;
        let mut buf = from_elem(0, size as usize);
        value.read(buf.as_mut_slice())?;

        Ok(Self(BigInt::from_signed_bytes_be(&buf)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_i256(number: &i256) {
        let encoded = scale::Encode::encode(&number);
        let decoded: i256 = scale::Decode::decode(&mut encoded.as_slice()).unwrap();
        assert_eq!(number, &decoded);
    }
    #[test]
    fn i256_codec() {
        let origin: i256 = (-1).into();
        let encoded = scale::Encode::encode(&origin);
        let decoded: i256 = scale::Decode::decode(&mut encoded.as_slice()).unwrap();
        assert!(origin == decoded);
        let number: i256 = i256::from(-1);
        check_i256(&number);
        let number: i256 = i256::from(-127);
        check_i256(&number);
        let number: i256 = i256::from(-128);
        check_i256(&number);
        let number: i256 = i256::from(-129);
        check_i256(&number);
        let number: i256 = i256::from(-255);
        check_i256(&number);
        let number: i256 = i256::from(-256);
        check_i256(&number);
        let number: i256 = i256::from(-257);
        check_i256(&number);
        let number: i256 = i256::from(-65535);
        check_i256(&number);
        let number: i256 = i256::from(-65536);
        check_i256(&number);
        let number: i256 = i256::from(-65537);
        check_i256(&number);
        let number: i256 = i256::from(-2147483647);
        check_i256(&number);

        let number: i256 = i256::from(1);
        check_i256(&number);
        let number: i256 = i256::from(127);
        check_i256(&number);
        let number: i256 = i256::from(128);
        check_i256(&number);
        let number: i256 = i256::from(129);
        check_i256(&number);
        let number: i256 = i256::from(255);
        check_i256(&number);
        let number: i256 = i256::from(256);
        check_i256(&number);
        let number: i256 = i256::from(257);
        check_i256(&number);
        let number: i256 = i256::from(65535);
        check_i256(&number);
        let number: i256 = i256::from(65536);
        check_i256(&number);
        let number: i256 = i256::from(65537);
        check_i256(&number);
        let number: i256 = i256::from(2147483647);
        check_i256(&number);
    }

    #[test]
    fn from_str() {
        let i1: i256 = "42258578699988885".parse().unwrap();
        let i2: i256 = "-0x9621F414202F95".parse().unwrap();
        assert_eq!(-i1, i2);
    }
}
