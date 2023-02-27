use crate::algorithms::eea::*;
use crate::euclidean::EuclideanRing;
use crate::field::Field;
use crate::{divisibility::*, Exists};
use crate::primitive::StaticRing;
use crate::ring::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ZnBase<const N: u64, const IS_FIELD: bool>;

pub const fn is_prime(n: u64) -> bool {
    assert!(n >= 2);
    let mut d = 2;
    while d < n {
        if n % d == 0 {
            return false;
        }
        d += 1;
    }
    return true;
}

impl<const N: u64, const IS_FIELD: bool> RingBase for ZnBase<N, IS_FIELD> 
    where [(); N as i64 as usize]: Exists
{
    
    type Element = u64;

    fn add_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) {
        *lhs += rhs;
        if *lhs >= N {
            *lhs -= N;
        }
    }

    fn negate_inplace(&self, lhs: &mut Self::Element) {
        if *lhs != 0 {
            *lhs = N - *lhs;
        }
    }

    fn mul_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) {
        *lhs = ((*lhs as u128 * rhs as u128) % (N as u128)) as u64
    }

    fn from_z(&self, value: i32) -> Self::Element {
        let result = ((value as i64 % (N as i64)) + (N as i64)) as u64;
        if result >= N {
            result - N
        } else {
            result
        }
    }

    fn eq(&self, lhs: &Self::Element, rhs: &Self::Element) -> bool {
        *lhs == *rhs
    }
    
    fn is_commutative(&self) -> bool { true }

    fn is_noetherian(&self) -> bool { true }

}

impl<const N: u64, const IS_FIELD: bool> DivisibilityRing for ZnBase<N, IS_FIELD> 
    where [(); N as i64 as usize]: Exists
{
    fn checked_div(&self, lhs: &Self::Element, rhs: &Self::Element) -> Option<Self::Element> {
        let (s, _, d) = signed_eea(*rhs as i64, N as i64, StaticRing::<i64>::RING);
        let mut rhs_inv = ((s % (N as i64)) + (N as i64)) as u64;
        if rhs_inv >= N {
            rhs_inv -= N;
        }
        if d == 1 {
            Some(self.mul(*lhs, rhs_inv))
        } else {
            None
        }
    }
}

impl<const N: u64> EuclideanRing for ZnBase<N, true> 
    where [(); N as i64 as usize]: Exists
{
    fn euclidean_div_rem(&self, lhs: Self::Element, rhs: &Self::Element) -> (Self::Element, Self::Element) {
        assert!(!self.is_zero(rhs));
        (self.checked_div(&lhs, rhs).unwrap(), self.zero())
    }

    fn euclidean_deg(&self, val: &Self::Element) -> usize {
        if self.is_zero(val) {
            0
        } else {
            1
        }
    }
}

impl<const N: u64> Field for ZnBase<N, true> 
    where [(); N as i64 as usize]: Exists
{}

impl<const N: u64, const IS_FIELD: bool> RingValue<ZnBase<N, IS_FIELD>> 
    where [(); N as i64 as usize]: Exists
{
    pub const RING: Self = Self::new(ZnBase);
}

pub type Zn<const N: u64> = RingValue<ZnBase<N, {is_prime(N)}>>;

#[test]
fn test_is_prime() {
    assert_eq!(true, is_prime(17));
    assert_eq!(false, is_prime(49));
}

pub const Z17: Zn<17> = Zn::<17>::RING;

#[test]
fn test_zn_el_add() {
    let a = Z17.from_z(6);
    let b = Z17.from_z(12);
    assert_eq!(Z17.from_z(1), Z17.add(a, b));
}

#[test]
fn test_zn_el_sub() {
    let a = Z17.from_z(6);
    let b = Z17.from_z(12);
    assert_eq!(Z17.from_z(11), Z17.sub(a, b));
}

#[test]
fn test_zn_el_mul() {
    let a = Z17.from_z(6);
    let b = Z17.from_z(12);
    assert_eq!(Z17.from_z(4), Z17.mul(a, b));
}

#[test]
fn test_zn_el_div() {
    let a = Z17.from_z(6);
    let b = Z17.from_z(12);
    assert_eq!(Z17.from_z(9), Z17.checked_div(&a, &b).unwrap());
}

#[test]
fn fn_test_div_impossible() {
    let _a = Zn::<22>::RING.from_z(4);
    // the following line should give a compiler error
    // Zn::<22>::RING.div(_a, _a);
}
