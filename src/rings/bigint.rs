use crate::divisibility::DivisibilityRing;
use crate::euclidean::*;
use crate::integer::*;
use crate::ordered::*;
use crate::primitive_int::StaticRingBase;
use crate::{ring::*};
use crate::algorithms;
use crate::primitive_int::*;
use std::cmp::Ordering::*;

#[derive(Clone, Debug)]
pub struct DefaultBigIntRingEl(bool, Vec<u64>);

impl DefaultBigIntRingEl {
    
    pub fn parse(string: &str, base: u32) -> Result<DefaultBigIntRingEl, ()> {
        let result = Vec::new();
        let (negative, rest) = if string.chars().next() == Some('-') {
            (true, string.split_at(1).1)
        } else if string.chars().next() == Some('+') {
            (false, string.split_at(1).1)
        } else {
            (false, string)
        };
        Ok(DefaultBigIntRingEl(negative, algorithms::bigint::from_str_radix(rest, base, result)?))
    }
}

#[derive(Copy, Clone)]
pub struct DefaultBigIntRing;

impl DefaultBigIntRing {

    pub const RING: RingValue<DefaultBigIntRing> = RingValue::from(DefaultBigIntRing);

    pub fn map_i128(&self, val: &<Self as RingBase>::Element) -> Option<i128> {
        match algorithms::bigint::highest_set_block(&val.1) {
            None => Some(0),
            Some(0) if val.0 => Some(-(val.1[0] as i128)),
            Some(0) if !val.0 => Some(val.1[0] as i128),
            Some(1) if val.0 => {
                let value = val.1[0] as u128 + ((val.1[1] as u128) << u64::BITS);
                if value == 1 << (u128::BITS - 1) {
                    Some(i128::MIN)
                } else {
                    i128::try_from(value).ok().map(|x| -x)
                }
            },
            Some(1) if !val.0 => i128::try_from(val.1[0] as u128 + ((val.1[1] as u128) << u64::BITS)).ok(),
            Some(_) => None
        }
    }
}

impl RingBase for DefaultBigIntRing {
    
    type Element = DefaultBigIntRingEl;

    fn add_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) {
        match (lhs, rhs) {
            (DefaultBigIntRingEl(false, lhs_val), DefaultBigIntRingEl(false, rhs_val)) |
            (DefaultBigIntRingEl(true, lhs_val), DefaultBigIntRingEl(true, rhs_val)) => {
                algorithms::bigint::bigint_add(lhs_val, rhs_val, 0);
            },
            (DefaultBigIntRingEl(lhs_sgn, lhs_val), DefaultBigIntRingEl(_, rhs_val)) => {
                match algorithms::bigint::bigint_cmp(lhs_val, rhs_val) {
                    Less => {
                        algorithms::bigint::bigint_sub_self(lhs_val, rhs_val);
                        *lhs_sgn = !*lhs_sgn;
                    },
                    Equal => {
                        lhs_val.clear();
                    },
                    Greater => {
                        algorithms::bigint::bigint_sub(lhs_val, rhs_val, 0);
                    }
                }
            }
        }
    }

    fn add_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) {
        self.add_assign_ref(lhs, &rhs);
    }

    fn sub_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) { 
        self.negate_inplace(lhs);
        self.add_assign_ref(lhs, rhs);
        self.negate_inplace(lhs);
    }

    fn negate_inplace(&self, lhs: &mut Self::Element) {
        lhs.0 = !lhs.0;
    }

    fn mul_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) {
        self.mul_assign_ref(lhs, &rhs);
    }

    fn mul_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) {
        let result = algorithms::bigint::bigint_mul(&lhs.1, &rhs.1, Vec::new());
        *lhs = DefaultBigIntRingEl(lhs.0 ^ rhs.0, result);
    }

    fn zero(&self) -> Self::Element {
        DefaultBigIntRingEl(false, Vec::new())
    }

    fn one(&self) -> Self::Element {
        DefaultBigIntRingEl(false, vec![1])
    }

    fn neg_one(&self) -> Self::Element {
        self.negate(self.one())
    }

    fn from_z(&self, value: i32) -> Self::Element {
        DefaultBigIntRingEl(value < 0, vec![(value as i64).abs() as u64])
    }

    fn eq(&self, lhs: &Self::Element, rhs: &Self::Element) -> bool {
        if lhs.0 == rhs.0 {
            algorithms::bigint::bigint_cmp(&lhs.1, &rhs.1) == Equal
        } else {
            self.is_zero(lhs) && self.is_zero(rhs)
        }
    }

    fn is_zero(&self, value: &Self::Element) -> bool {
        algorithms::bigint::highest_set_block(&value.1).is_none()
    }

    fn is_one(&self, value: &Self::Element) -> bool {
        value.0 == false && algorithms::bigint::highest_set_block(&value.1) == Some(0) && value.1[0] == 1
    }

    fn is_neg_one(&self, value: &Self::Element) -> bool {
        value.0 == true && algorithms::bigint::highest_set_block(&value.1) == Some(0) && value.1[0] == 1
    }

    fn is_commutative(&self) -> bool { true }
    fn is_noetherian(&self) -> bool { true }
    
    fn dbg<'a>(&self, value: &Self::Element, out: &mut std::fmt::Formatter<'a>) -> std::fmt::Result {
        ///
        /// 10 to this power fits still in a u64
        /// 
        const BIG_POWER_TEN_ZEROS: usize = 19;
        const BIG_POWER_TEN: u64 = 10u64.pow(BIG_POWER_TEN_ZEROS as u32);

        if value.0 {
            write!(out, "-")?;
        }
        let mut copy = value.clone();
        let mut remainders: Vec<u64> = Vec::with_capacity(
            (algorithms::bigint::highest_set_block(&value.1).unwrap_or(0) + 1) * u64::BITS as usize / 3
        );
        while !self.is_zero(&copy) {
            let rem = algorithms::bigint::bigint_div_small(&mut copy.1, BIG_POWER_TEN);
            remainders.push(rem);
        }
        remainders.reverse();
        let mut it = remainders.into_iter();
        if let Some(fst) = it.next() {
            write!(out, "{}", fst)?;
            for rem in it {
                write!(out, "{:0>width$}", rem, width = BIG_POWER_TEN_ZEROS)?;
            }
        } else {
            write!(out, "0")?;
        }
        return Ok(());
    }

}

impl CanonicalHom<DefaultBigIntRing> for DefaultBigIntRing {
    
    fn has_canonical_hom(&self, _: &DefaultBigIntRing) -> Option<()> { Some(()) }

    fn map_in(&self, _: &DefaultBigIntRing, el: DefaultBigIntRingEl, _: &()) -> Self::Element { el }
}

impl CanonicalIso<DefaultBigIntRing> for DefaultBigIntRing {
    
    fn has_canonical_iso(&self, _: &DefaultBigIntRing) -> Option<()> { Some(()) }

    fn map_out(&self, _: &DefaultBigIntRing, el: DefaultBigIntRingEl, _: &()) -> Self::Element { el }
}

impl OrderedRing for DefaultBigIntRing {

    fn cmp(&self, lhs: &Self::Element, rhs: &Self::Element) -> std::cmp::Ordering {
        match (lhs.0, rhs.0) {
            (true, true) => algorithms::bigint::bigint_cmp(&rhs.1, &lhs.1),
            (false, false) => algorithms::bigint::bigint_cmp(&lhs.1, &rhs.1),
            (_, _) if self.is_zero(lhs) && self.is_zero(rhs) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater
        }
    }
}

impl DivisibilityRing for DefaultBigIntRing {
    
    fn checked_left_div(&self, lhs: &Self::Element, rhs: &Self::Element) -> Option<Self::Element> {
        if self.is_zero(rhs) && self.is_zero(lhs) {
            return Some(self.zero());
        } else if self.is_zero(rhs) {
            return None;
        }
        let (quo, rem) = self.euclidean_div_rem(lhs.clone(), rhs);
        if self.is_zero(&rem) {
            Some(quo)
        } else {
            None
        }
    }
}

impl EuclideanRing for DefaultBigIntRing {

    fn euclidean_div_rem(&self, mut lhs: Self::Element, rhs: &Self::Element) -> (Self::Element, Self::Element) {
        assert!(!self.is_zero(rhs));
        let mut quo = DefaultBigIntRingEl(false, algorithms::bigint::bigint_div(&mut lhs.1, &rhs.1, Vec::new()));
        if rhs.0 ^ lhs.0 {// if result of division is zero, `.is_neg(&lhs)` does not work as expected
            self.negate_inplace(&mut quo);
        }
        return (quo, lhs);
    }

    fn euclidean_deg(&self, val: &Self::Element) -> Option<usize> {
        self.map_i128(val).and_then(|x| x.checked_abs()).and_then(|x| usize::try_from(x).ok())
    }
}

impl<T: PrimitiveInt> CanonicalHom<StaticRingBase<T>> for DefaultBigIntRing {
    
    fn has_canonical_hom(&self, _: &StaticRingBase<T>) -> Option<()> { Some(()) }

    fn map_in(&self, _: &StaticRingBase<T>, el: T, _: &()) -> Self::Element {
        let negative = el.into() < 0;
        let value = el.into().checked_abs().map(|x| x as u128).unwrap_or(1 << (u128::BITS - 1));
        DefaultBigIntRingEl(negative, vec![(value & ((1 << u64::BITS) - 1)) as u64, (value >> u64::BITS) as u64])
    }
}

impl<T: PrimitiveInt> CanonicalIso<StaticRingBase<T>> for DefaultBigIntRing {
    
    fn has_canonical_iso(&self, _: &StaticRingBase<T>) -> Option<()> { Some(()) }

    fn map_out(&self, _: &StaticRingBase<T>, el: Self::Element, _: &()) -> T {
        T::try_from(self.map_i128(&el).unwrap()).ok().unwrap()
    }
}

impl HashableElRing for DefaultBigIntRing {

    fn hash<H: std::hash::Hasher>(&self, el: &Self::Element, h: &mut H) {
        let block = algorithms::bigint::highest_set_block(&el.1);
        if let Some(b) = block {
            for i in 0..=b {
                h.write_u64(el.1[i])
            }
        }
    }
}

impl IntegerRing for DefaultBigIntRing {

    fn to_float_approx(&self, value: &Self::Element) -> f64 {
        let sign = if value.0 { -1. } else { 1. };
        match algorithms::bigint::highest_set_block(&value.1) {
            None => 0.,
            Some(0) => value.1[0] as f64 * sign,
            Some(d) => (value.1[d] as f64 * 2f64.powi(d as i32 * u64::BITS as i32) + value.1[d - 1] as f64 * 2f64.powi((d - 1) as i32 * u64::BITS as i32)) * sign
        }
    }

    fn from_float_approx(&self, mut value: f64) -> Option<Self::Element> {
        if value.round() == 0. {
            return Some(self.zero());
        }
        let sign = value < 0.;
        value = value.abs();
        let scale = value.log2().ceil() as i32;
        let significant_digits = std::cmp::min(scale, u64::BITS as i32);
        let most_significant_bits = (value / 2f64.powi(scale - significant_digits)) as u64;
        let mut result = DefaultBigIntRingEl(sign, vec![most_significant_bits]);
        self.mul_pow_2(&mut result, (scale - significant_digits) as usize);
        return Some(result);
    }

    fn abs_lowest_set_bit(&self, value: &Self::Element) -> Option<usize> {
        if self.is_zero(value) {
            return None;
        }
        for i in 0..value.1.len() {
            if value.1[i] != 0 {
                return Some(i * u64::BITS as usize + value.1[i].trailing_zeros() as usize)
            }
        }
        unreachable!()
    }

    fn abs_is_bit_set(&self, value: &Self::Element, i: usize) -> bool {
        if i / u64::BITS as usize >= value.1.len() {
            false
        } else {
            (value.1[i / u64::BITS as usize] >> (i % u64::BITS as usize)) & 1 == 1
        }
    }

    fn abs_highest_set_bit(&self, value: &Self::Element) -> Option<usize> {
        let block = algorithms::bigint::highest_set_block(&value.1)?;
        Some(block * u64::BITS as usize + u64::BITS as usize - value.1[block].leading_zeros() as usize - 1)
    }

    fn euclidean_div_pow_2(&self, value: &mut Self::Element, power: usize) {
        algorithms::bigint::bigint_rshift(&mut value.1, power);
    }

    fn mul_pow_2(&self, value: &mut Self::Element, power: usize) {
        algorithms::bigint::bigint_lshift(&mut value.1, power)
    }

    fn get_uniformly_random_bits<G: FnMut() -> u64>(&self, log2_bound_exclusive: usize, mut rng: G) -> Self::Element {
        let blocks = log2_bound_exclusive / u64::BITS as usize;
        let in_block = log2_bound_exclusive % u64::BITS as usize;
        if in_block == 0 {
            DefaultBigIntRingEl(false, (0..blocks).map(|_| rng()).collect())
        } else {
            let last = rng() & 1u64.overflowing_shl(in_block as u32).0.overflowing_sub(1).0;
            DefaultBigIntRingEl(false, (0..blocks).map(|_| rng()).chain(std::iter::once(last)).collect())
        }
    }
}

#[cfg(test)]
use crate::divisibility::test_divisibility_axioms;

#[test]
fn test_print_power_2() {
    let x = DefaultBigIntRingEl(false, vec![0, 0, 1]);
    assert_eq!("340282366920938463463374607431768211456", format!("{}", DefaultBigIntRing::RING.format(&x)));
}

#[test]
fn test_from() {
    assert!(DefaultBigIntRing::RING.eq(&DefaultBigIntRingEl(false, vec![]), &DefaultBigIntRing::RING.from_z(0)));
    assert!(DefaultBigIntRing::RING.eq(&DefaultBigIntRingEl(false, vec![2138479]), &DefaultBigIntRing::RING.from_z(2138479)));
    assert!(DefaultBigIntRing::RING.eq(&DefaultBigIntRingEl(true, vec![2138479]), &DefaultBigIntRing::RING.from_z(-2138479)));
    // assert!(DefaultBigIntRing::RING.eq(&DefaultBigInt(false, vec![0x38691a350bf12fca, 0x1]), &DefaultBigIntRing::RING.from_z_gen(0x138691a350bf12fca, &i128::RING)));
}

#[test]
fn test_to_i128() {
    assert_eq!(0, DefaultBigIntRing::RING.cast(&StaticRing::<i128>::RING, DefaultBigIntRingEl(false, vec![])));
    assert_eq!(2138479, DefaultBigIntRing::RING.cast(&StaticRing::<i128>::RING, DefaultBigIntRingEl(false, vec![2138479])));
    assert_eq!(-2138479, DefaultBigIntRing::RING.cast(&StaticRing::<i128>::RING, DefaultBigIntRingEl(true, vec![2138479])));
    assert_eq!(0x138691a350bf12fca, DefaultBigIntRing::RING.cast(&StaticRing::<i128>::RING, DefaultBigIntRingEl(false, vec![0x38691a350bf12fca, 0x1])));
    // assert_eq!(Err(()), DefaultBigInt(false, vec![0x38691a350bf12fca, 0x38691a350bf12fca, 0x1]).to_i128());
    assert_eq!(i128::MAX, DefaultBigIntRing::RING.cast(&StaticRing::<i128>::RING, DefaultBigIntRingEl(false, vec![(i128::MAX & ((1 << 64) - 1)) as u64, (i128::MAX >> 64) as u64])));
    assert_eq!(i128::MIN + 1, DefaultBigIntRing::RING.cast(&StaticRing::<i128>::RING, DefaultBigIntRingEl(true, vec![(i128::MAX & ((1 << 64) - 1)) as u64, (i128::MAX >> 64) as u64])));
    // this is the possibly surprising, exceptional case
    // assert_eq!(Err(()), DefaultBigInt(true, vec![0, (i128::MAX >> 64) as u64 + 1]).to_i128());
    assert_eq!(i64::MAX as i128 + 1, DefaultBigIntRing::RING.cast(&StaticRing::<i128>::RING, DefaultBigIntRingEl(false, vec![i64::MAX as u64 + 1])));
    assert_eq!(u64::MAX as i128, DefaultBigIntRing::RING.cast(&StaticRing::<i128>::RING, DefaultBigIntRingEl(false, vec![u64::MAX])));
}

#[test]
fn test_sub_assign() {
    let mut x = DefaultBigIntRingEl::parse("4294836225", 10).unwrap();
    let y =     DefaultBigIntRingEl::parse("4294967297", 10).unwrap();
    let z =     DefaultBigIntRingEl::parse("-131072", 10).unwrap();
    x = DefaultBigIntRing::RING.sub_ref_fst(&x, y);
    assert!(DefaultBigIntRing::RING.eq(&z, &x));
}

#[test]
fn test_shift_right() {
    let mut x = DefaultBigIntRingEl::parse("9843a756781b34567f81394", 16).unwrap();
    let z = DefaultBigIntRingEl::parse("9843a756781b34567", 16).unwrap();
    DefaultBigIntRing::RING.euclidean_div_pow_2(&mut x, 24);
    assert!(DefaultBigIntRing::RING.eq(&z, &x));

    let mut x = DefaultBigIntRingEl::parse("-9843a756781b34567f81394", 16).unwrap();
    let z = DefaultBigIntRingEl::parse("-9843a756781b34567", 16).unwrap();
    DefaultBigIntRing::RING.euclidean_div_pow_2(&mut x, 24);
    assert!(DefaultBigIntRing::RING.eq(&z, &x));
}

#[test]
fn test_assumptions_integer_division() {
    assert_eq!(-1, -3 / 2);
    assert_eq!(-1, 3 / -2);
    assert_eq!(1, -3 / -2);
    assert_eq!(1, 3 / 2);

    assert_eq!(-1, -3 % 2);
    assert_eq!(1, 3 % -2);
    assert_eq!(-1, -3 % -2);
    assert_eq!(1, 3 % 2);
}

#[cfg(test)]
fn numbers_iter() -> impl Iterator<Item = DefaultBigIntRingEl> {
    
    const NUMBERS: [&'static str; 10] = [
        "5444517870735015415413993718908291383295", // power of two - 1
        "5444517870735015415413993718908291383296", // power of two
        "-5444517870735015415413993718908291383295",
        "-5444517870735015415413993718908291383296",
        "3489", // the rest is random
        "891023591340178345678931246518793456983745682137459364598623489512389745698237456890239238476873429872346579",
        "172365798123602365091834765607185713205612370956192783561461248973265193754762751378496572896497125361819754136",
        "0",
        "-231780567812394562346324763251741827457123654871236548715623487612384752328164",
        "+1278367182354612381234568509783420989356938472561078564732895634928563482349872698723465"
    ];

    NUMBERS.iter().cloned().map(|s| DefaultBigIntRingEl::parse(s, 10)).map(Result::unwrap)
}

#[test]
fn test_bigint_ring_axioms() {
    test_ring_axioms(DefaultBigIntRing::RING, numbers_iter())
}

#[test]
fn test_bigint_divisibility_ring_axioms() {
    test_divisibility_axioms(DefaultBigIntRing::RING, numbers_iter())
}

#[test]
fn test_bigint_euclidean_ring_axioms() {
    test_euclidean_axioms(DefaultBigIntRing::RING, numbers_iter())
}

#[test]
fn test_bigint_integer_ring_axioms() {
    test_integer_axioms(DefaultBigIntRing::RING, numbers_iter())
}

#[bench]
fn bench_mul(bencher: &mut test::Bencher) {
    let x = DefaultBigIntRingEl::parse("2382385687561872365981723456981723456987134659834659813491964132897159283746918732563498628754", 10).unwrap();
    let y = DefaultBigIntRingEl::parse("48937502893645789234569182735646324895723409587234", 10).unwrap();
    let z = DefaultBigIntRingEl::parse("116588006478839442056346504147013274749794691549803163727888681858469844569693215953808606899770104590589390919543097259495176008551856143726436", 10).unwrap();
    bencher.iter(|| {
        let p = DefaultBigIntRing::RING.mul_ref(&x, &y);
        assert!(DefaultBigIntRing::RING.eq(&z, &p));
    })
}

#[test]
fn from_to_float_approx() {
    let x: f64 = 83465209236517892563478156042389675783219532497861237985328563.;
    let y = DefaultBigIntRing::RING.to_float_approx(&DefaultBigIntRing::RING.from_float_approx(x).unwrap());
    assert!(x * 0.99 < y);
    assert!(y < x * 1.01);
}

#[bench]
fn bench_div(bencher: &mut test::Bencher) {
    let x = DefaultBigIntRingEl::parse("2382385687561872365981723456981723456987134659834659813491964132897159283746918732563498628754", 10).unwrap();
    let y = DefaultBigIntRingEl::parse("48937502893645789234569182735646324895723409587234", 10).unwrap();
    let z = DefaultBigIntRingEl::parse("48682207850683149082203680872586784064678018", 10).unwrap();
    bencher.iter(|| {
        let q = DefaultBigIntRing::RING.euclidean_div(x.clone(), &y);
        assert!(DefaultBigIntRing::RING.eq(&z, &q));
    })
}

// #[test]
// fn test_eq() {
//     fn calculate_hash<T: Hash>(t: &T) -> u64 {
//         let mut s = std::collections::hash_map::DefaultHasher::new();
//         t.hash(&mut s);
//         s.finish()
//     }

//     let a = DefaultBigInt(false, vec![98711]);
//     let b = DefaultBigInt(false, vec![98711, 0]);
//     assert!(a == 98711);
//     assert!(a == b);
//     assert!(b == a);
//     assert!(a != DefaultBigIntRing::RING.neg(a.clone()));
//     assert!(calculate_hash(&a) == calculate_hash(&b));
//     assert!(a != DefaultBigIntRing::RING.one());
//     // the next line could theoretically fail, but it is very improbable and we definitly should test hash inequality
//     assert!(calculate_hash(&a) != calculate_hash(&DefaultBigIntRing::RING.one()));
// }

#[test]
fn test_is_zero() {
    let zero = DefaultBigIntRing::RING.zero();
    let mut nonzero = DefaultBigIntRing::RING.one();
    DefaultBigIntRing::RING.mul_pow_2(&mut nonzero, 83124);
    assert!(DefaultBigIntRing::RING.is_zero(&zero));
    assert!(DefaultBigIntRing::RING.is_zero(&DefaultBigIntRing::RING.negate(zero)));
    assert!(!DefaultBigIntRing::RING.is_zero(&nonzero));
    assert!(!DefaultBigIntRing::RING.is_zero(&DefaultBigIntRing::RING.negate(nonzero)));
}

// #[test]
// fn test_cmp_small() {
//     assert!(DefaultBigInt::parse("-23678", 10).unwrap() < 0);
// }

#[test]
fn test_cmp() {
    assert_eq!(true, DefaultBigIntRing::RING.is_lt(&DefaultBigIntRing::RING.from_z(-1), &DefaultBigIntRing::RING.from_z(2)));
    assert_eq!(true, DefaultBigIntRing::RING.is_lt(&DefaultBigIntRing::RING.from_z(1), &DefaultBigIntRing::RING.from_z(2)));
    assert_eq!(false, DefaultBigIntRing::RING.is_lt(&DefaultBigIntRing::RING.from_z(2), &DefaultBigIntRing::RING.from_z(2)));
    assert_eq!(false, DefaultBigIntRing::RING.is_lt(&DefaultBigIntRing::RING.from_z(3), &DefaultBigIntRing::RING.from_z(2)));
    assert_eq!(true, DefaultBigIntRing::RING.is_gt(&DefaultBigIntRing::RING.from_z(-1), &DefaultBigIntRing::RING.from_z(-2)));
}

#[test]
fn test_mul_pow_2() {
    let mut x = DefaultBigIntRing::RING.from_z(2);
    DefaultBigIntRing::RING.mul_pow_2(&mut x, 0);
    assert!(DefaultBigIntRing::RING.eq(&DefaultBigIntRing::RING.from_z(2), &x));

    let mut x = DefaultBigIntRing::RING.from_z(4829192);
    DefaultBigIntRing::RING.mul_pow_2(&mut x, 3);
    assert!(DefaultBigIntRing::RING.eq(&DefaultBigIntRing::RING.from_z(4829192 * 8), &x));

    let mut x = DefaultBigIntRing::RING.one();
    DefaultBigIntRing::RING.mul_pow_2(&mut x, 64);
    DefaultBigIntRing::RING.mul_assign(&mut x, DefaultBigIntRing::RING.from_z(4829192));

    let mut z = DefaultBigIntRing::RING.from_z(4829192);
    DefaultBigIntRing::RING.mul_pow_2(&mut z, 64);
    assert!(DefaultBigIntRing::RING.eq(&x, &z));
}

#[test]
fn test_get_uniformly_random() {
    test_integer_uniformly_random(DefaultBigIntRing::RING);

    let ring = DefaultBigIntRing::RING;
    let bound = DefaultBigIntRingEl::parse("11000000000000000", 16).unwrap();
    let block_bound = DefaultBigIntRingEl::parse("10000000000000000", 16).unwrap();
    let mut rng = oorandom::Rand64::new(0);
    let elements: Vec<_> = (0..1000).map(|_| ring.get_uniformly_random(&bound, || rng.rand_u64())).collect();
    assert!(elements.iter().any(|x| ring.is_lt(x, &block_bound)));
    assert!(elements.iter().any(|x| ring.is_gt(x, &block_bound)));
    assert!(elements.iter().all(|x| ring.is_lt(x, &bound)));
}