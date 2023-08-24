use crate::ring::*;
use crate::euclidean::*;
use crate::ordered::*;
use crate::primitive_int::*;

#[cfg(feature = "mpir")]
pub type BigIntRing = crate::rings::mpir::MPZ;
#[cfg(not(feature = "mpir"))]
pub type BigIntRing = crate::rings::rust_bigint::RustBigintRing;

///
/// Trait for rings that are isomorphic to the ring of integers `ZZ = { ..., -2, -1, 0, 1, 2, ... }`.
/// 
/// Some of the functionality in this trait refers to the binary expansion of
/// a positive integer. While this is not really general, it is often required
/// for fast operations with integers.
/// 
/// As an additional requirement, the euclidean division (i.e. [`EuclideanRing::euclidean_div_rem()`] and
/// [`IntegerRing::euclidean_div_pow_2()`]) are expected to round towards zero.
/// 
pub trait IntegerRing: EuclideanRing + OrderedRing + HashableElRing + SelfIso + CanonicalIso<StaticRingBase<i32>> {

    ///
    /// Computes a float value that is supposed to be close to value.
    /// However, no guarantees are made on how close it must be, in particular,
    /// this function may also always return `0.`. It is supposed to be used for
    /// optimization purposes only, e.g. in the case where an approximate value is
    /// necessary to determine performance-controlling parameters, or as an initial
    /// value for some iterative root-finding algorithm.
    /// 
    fn to_float_approx(&self, value: &Self::Element) -> f64;

    ///
    /// Computes a value that is "close" to the given float. However, no guarantees
    /// are made on the definition of close, in particular, this does not have to be
    /// the closest integer to the given float, and cannot be used to compute rounding.
    /// It is also implementation-defined when to return `None`, although this is usually
    /// the case on infinity and NaN.
    /// 
    fn from_float_approx(&self, value: f64) -> Option<Self::Element>;

    ///
    /// Return whether the `i`-th bit in the two-complements representation of `abs(value)`
    /// is `1`.
    /// 
    /// # Example
    /// ```
    /// # use feanor_math::primitive_int::*;
    /// # use feanor_math::integer::*;
    /// # use feanor_math::ring::*;
    /// assert_eq!(false, StaticRing::<i32>::RING.abs_is_bit_set(&4, 1));
    /// assert_eq!(true, StaticRing::<i32>::RING.abs_is_bit_set(&4, 2));
    /// assert_eq!(true, StaticRing::<i32>::RING.abs_is_bit_set(&-4, 2));
    /// ```
    /// 
    fn abs_is_bit_set(&self, value: &Self::Element, i: usize) -> bool;

    ///
    /// Returns the index of the highest set bit in the two-complements representation of `abs(value)`,
    /// or `None` if the value is zero.
    /// 
    /// # Example
    /// ```
    /// # use feanor_math::primitive_int::*;
    /// # use feanor_math::integer::*;
    /// # use feanor_math::ring::*;
    /// assert_eq!(None, StaticRing::<i32>::RING.abs_highest_set_bit(&0));
    /// assert_eq!(Some(0), StaticRing::<i32>::RING.abs_highest_set_bit(&-1));
    /// assert_eq!(Some(2), StaticRing::<i32>::RING.abs_highest_set_bit(&4));
    /// ```
    /// 
    fn abs_highest_set_bit(&self, value: &Self::Element) -> Option<usize>;

    ///
    /// Returns the index of the lowest set bit in the two-complements representation of `abs(value)`,
    /// or `None` if the value is zero.
    /// 
    /// # Example
    /// ```
    /// # use feanor_math::primitive_int::*;
    /// # use feanor_math::integer::*;
    /// # use feanor_math::ring::*;
    /// assert_eq!(None, StaticRing::<i32>::RING.abs_lowest_set_bit(&0));
    /// assert_eq!(Some(0), StaticRing::<i32>::RING.abs_lowest_set_bit(&1));
    /// assert_eq!(Some(0), StaticRing::<i32>::RING.abs_lowest_set_bit(&-3));
    /// assert_eq!(Some(2), StaticRing::<i32>::RING.abs_lowest_set_bit(&4));
    /// ```
    /// 
    fn abs_lowest_set_bit(&self, value: &Self::Element) -> Option<usize>;

    ///
    /// Computes the euclidean division by a power of two, always rounding to zero (note that
    /// euclidean division requires that `|remainder| < |divisor|`, and thus would otherwise
    /// leave multiple possible results).
    /// 
    /// # Example
    /// ```
    /// # use feanor_math::primitive_int::*;
    /// # use feanor_math::integer::*;
    /// # use feanor_math::ring::*;
    /// let mut value = -7;
    /// StaticRing::<i32>::RING.euclidean_div_pow_2(&mut value, 1);
    /// assert_eq!(-3, value);
    /// ```
    /// 
    fn euclidean_div_pow_2(&self, value: &mut Self::Element, power: usize);

    ///
    /// Multiplies the element by a power of two.
    /// 
    fn mul_pow_2(&self, value: &mut Self::Element, power: usize);

    ///
    /// Computes a uniformly random integer in `[0, 2^log_bound_exclusive - 1]`, assuming that
    /// `rng` provides uniformly random values in the whole range of `u64`.
    /// 
    fn get_uniformly_random_bits<G: FnMut() -> u64>(&self, log2_bound_exclusive: usize, rng: G) -> Self::Element;

    ///
    /// Computes the rounded division, i.e. rounding to the closest integer.
    /// In the case of a tie (i.e. `round(0.5)`), we round towards `+/- infinity`.
    /// 
    /// # Example
    /// ```
    /// # use feanor_math::primitive_int::*;
    /// # use feanor_math::integer::*;
    /// # use feanor_math::ring::*;
    /// assert_eq!(2, StaticRing::<i32>::RING.rounded_div(7, &3));
    /// assert_eq!(-2, StaticRing::<i32>::RING.rounded_div(-7, &3));
    /// assert_eq!(-2, StaticRing::<i32>::RING.rounded_div(7, &-3));
    /// assert_eq!(2, StaticRing::<i32>::RING.rounded_div(-7, &-3));
    /// 
    /// assert_eq!(3, StaticRing::<i32>::RING.rounded_div(8, &3));
    /// assert_eq!(-3, StaticRing::<i32>::RING.rounded_div(-8, &3));
    /// assert_eq!(-3, StaticRing::<i32>::RING.rounded_div(8, &-3));
    /// assert_eq!(3, StaticRing::<i32>::RING.rounded_div(-8, &-3));
    /// 
    /// assert_eq!(4, StaticRing::<i32>::RING.rounded_div(7, &2));
    /// assert_eq!(-4, StaticRing::<i32>::RING.rounded_div(-7, &2));
    /// assert_eq!(-4, StaticRing::<i32>::RING.rounded_div(7, &-2));
    /// assert_eq!(4, StaticRing::<i32>::RING.rounded_div(-7, &-2));
    /// ```
    /// 
    fn rounded_div(&self, lhs: Self::Element, rhs: &Self::Element) -> Self::Element {
        let mut rhs_half = self.abs(self.clone_el(rhs));
        self.euclidean_div_pow_2(&mut rhs_half, 1);
        if self.is_neg(&lhs) {
            return self.euclidean_div(self.sub(lhs, rhs_half), rhs);
        } else {
            return self.euclidean_div(self.add(lhs, rhs_half), rhs);
        }
    }

    fn power_of_two(&self, power: usize) -> Self::Element {
        let mut result = self.one();
        self.mul_pow_2(&mut result, power);
        return result;
    }
}

///
/// Helper trait to simplify conversion between ints.
/// While for all integer rings `I`, `J`, one usually has `I: CanonicalIso<J>`,
/// this might not be the case for rings in different crates, or it is too cumbersome
/// to include the trait bounds when writing generic code.
/// `IntCast` allows the conversion between arbitrary integer rings, using a
/// "double-and-add"-approach. However, if `I: CanonicalHom<J>` is implemented, a
/// specialization uses the (hopefully more efficient) variant. 
/// 
pub trait IntCast<F: ?Sized + IntegerRing>: IntegerRing {

    fn cast(&self, from: &F, value: F::Element) -> Self::Element;
}

impl<F: ?Sized + IntegerRing, T: ?Sized + IntegerRing> IntCast<F> for T {

    default fn cast(&self, from: &F, value: F::Element) -> Self::Element {
        generic_maps::generic_map_in(from, self, value)
    }
}

impl<F: ?Sized + IntegerRing, T: ?Sized + IntegerRing> IntCast<F> for T
    where T: CanonicalHom<F>
{
    default fn cast(&self, from: &F, value: F::Element) -> Self::Element {
        self.map_in(from, value, &self.has_canonical_hom(from).unwrap())
    }
}

pub fn int_cast<T: IntegerRingStore, F: IntegerRingStore>(value: El<F>, to: T, from: F) -> El<T>
    where T::Type: IntegerRing, F::Type: IntegerRing
{
    <T::Type as IntCast<F::Type>>::cast(to.get_ring(), from.get_ring(), value)
}

pub mod generic_maps {
    use crate::{algorithms, ring::{RingRef, RingBase}};
    use super::IntegerRing;

    pub fn generic_map_in<R: ?Sized + IntegerRing, S: ?Sized + RingBase>(from: &R, to: &S, el: R::Element) -> S::Element {
        let result = algorithms::sqr_mul::generic_abs_square_and_multiply(to.one(), &el, RingRef::new(from), |a| to.add_ref(&a, &a), |a, b| to.add_ref_fst(a, b), to.zero());
        if from.is_neg(&el) {
            return to.negate(result);
        } else {
            return result;
        }
    }
}

pub trait IntegerRingStore: RingStore
    where Self::Type: IntegerRing
{
    delegate!{ fn to_float_approx(&self, value: &El<Self>) -> f64 }
    delegate!{ fn from_float_approx(&self, value: f64) -> Option<El<Self>> }
    delegate!{ fn abs_is_bit_set(&self, value: &El<Self>, i: usize) -> bool }
    delegate!{ fn abs_highest_set_bit(&self, value: &El<Self>) -> Option<usize> }
    delegate!{ fn abs_lowest_set_bit(&self, value: &El<Self>) -> Option<usize> }
    delegate!{ fn euclidean_div_pow_2(&self, value: &mut El<Self>, power: usize) -> () }
    delegate!{ fn mul_pow_2(&self, value: &mut El<Self>, power: usize) -> () }
    delegate!{ fn power_of_two(&self, power: usize) -> El<Self> }
    delegate!{ fn rounded_div(&self, lhs: El<Self>, rhs: &El<Self>) -> El<Self> }

    fn get_uniformly_random<G: FnMut() -> u64>(&self, bound_exclusive: &El<Self>, mut rng: G) -> El<Self> {
        assert!(self.is_gt(bound_exclusive, &self.zero()));
        let log2_ceil_bound = self.abs_highest_set_bit(bound_exclusive).unwrap() + 1;
        let mut result = self.get_ring().get_uniformly_random_bits(log2_ceil_bound, || rng());
        while self.is_geq(&result, bound_exclusive) {
            result = self.get_ring().get_uniformly_random_bits(log2_ceil_bound, || rng());
        }
        return result;
    }

    fn abs_log2_ceil(&self, value: &El<Self>) -> Option<usize> {
        let highest_bit = self.abs_highest_set_bit(value)?;
        if self.abs_lowest_set_bit(value).unwrap() == highest_bit {
            return Some(highest_bit);
        } else {
            return Some(highest_bit + 1);
        }
    }

    fn is_even(&self, value: &El<Self>) -> bool {
        !self.abs_is_bit_set(value, 0)
    }

    fn is_odd(&self, value: &El<Self>) -> bool {
        !self.is_even(value)
    }

    fn half_exact(&self, mut value: El<Self>) -> El<Self> {
        assert!(self.is_even(&value));
        self.euclidean_div_pow_2(&mut value, 1);
        return value;
    }
}

impl<R> IntegerRingStore for R
    where R: RingStore,
        R::Type: IntegerRing
{}

pub mod generic_impls {
    use crate::{algorithms, ring::RingRef};

    use super::IntegerRing;

    pub fn generic_map_in<I: ?Sized + IntegerRing, J: ?Sized + IntegerRing>(from: &I, to: &J, el: &I::Element) -> J::Element {
        algorithms::sqr_mul::generic_abs_square_and_multiply(
            to.one(), 
            el, 
            RingRef::new(from), 
            |a| to.add_ref(&a, &a), 
            |a, b| to.add_ref_fst(a, b), 
            to.zero()
        )
    }
}

#[cfg(test)]
pub fn generic_test_integer_uniformly_random<R: IntegerRingStore>(ring: R) 
    where R::Type: IntegerRing
{
    for b in [15, 16] {
        let bound = ring.from_int(b);
        let mut rng = oorandom::Rand64::new(0);
        let elements: Vec<El<R>> = (0..1000).map(|_| ring.get_uniformly_random(&bound, || rng.rand_u64())).collect();
        for i in 0..b {
            assert!(elements.iter().any(|x| ring.eq_el(x, &ring.from_int(i))))
        }
        for x in &elements {
            assert!(ring.is_lt(x, &bound));
        }
    }
}

#[cfg(any(test, feature = "generic_tests"))]
pub fn generic_test_integer_axioms<R: IntegerRingStore, I: Iterator<Item = El<R>>>(ring: R, edge_case_elements: I) 
    where R::Type: IntegerRing
{
    let elements = edge_case_elements.collect::<Vec<_>>();

    // test abs_highest_set_bit on standard values
    assert_eq!(None, ring.abs_highest_set_bit(&ring.from_int(0)));
    assert_eq!(Some(0), ring.abs_highest_set_bit(&ring.from_int(1)));
    assert_eq!(Some(1), ring.abs_highest_set_bit(&ring.from_int(2)));

    // generic test of mul_pow_2 resp. euclidean_div_pow_2
    for a in &elements {
        let mut ceil_pow_2 = ring.from_int(2);
        ring.mul_pow_2(&mut ceil_pow_2, ring.abs_highest_set_bit(a).unwrap_or(0));
        assert!(ring.is_lt(a, &ceil_pow_2));
        assert!(ring.is_lt(&ring.negate(ring.clone_el(a)), &ceil_pow_2));
        
        for i in 0..ring.abs_highest_set_bit(a).unwrap_or(0) {
            let mut pow_2 = ring.one();
            ring.mul_pow_2(&mut pow_2, i);
            let mut b = ring.clone_el(a);
            ring.mul_pow_2(&mut b, i);
            assert_el_eq!(&ring, &b, &ring.mul(ring.clone_el(a), ring.clone_el(&pow_2)));
            ring.euclidean_div_pow_2(&mut b, i);
            assert_el_eq!(&ring, &b, a);
            ring.euclidean_div_pow_2(&mut b, i);
            assert_el_eq!(&ring, &b, &ring.euclidean_div(ring.clone_el(a), &pow_2));
        }
    }

    // test euclidean div round to zero
    let d = ring.from_int(8);
    for k in -10..=10 {
        let mut a = ring.from_int(k);
        assert_el_eq!(&ring, &ring.from_int(k / 8), &ring.euclidean_div(ring.clone_el(&a), &d));
        ring.euclidean_div_pow_2(&mut a, 3);
        assert_el_eq!(&ring, &ring.from_int(k / 8), &a);
    }
    let d = ring.from_int(-8);
    for k in -10..=10 {
        let a = ring.from_int(k);
        assert_el_eq!(&ring, &ring.from_int(k / -8), &ring.euclidean_div(ring.clone_el(&a), &d));
    }

    // test rounded_div
    assert_el_eq!(&ring, &ring.from_int(2), &ring.rounded_div(ring.from_int(7), &ring.from_int(3)));
    assert_el_eq!(&ring, &ring.from_int(-2), &ring.rounded_div(ring.from_int(-7), &ring.from_int(3)));
    assert_el_eq!(&ring, &ring.from_int(-2), &ring.rounded_div(ring.from_int(7), &ring.from_int(-3)));
    assert_el_eq!(&ring, &ring.from_int(2), &ring.rounded_div(ring.from_int(-7), &ring.from_int(-3)));

    assert_el_eq!(&ring, &ring.from_int(3), &ring.rounded_div(ring.from_int(8), &ring.from_int(3)));
    assert_el_eq!(&ring, &ring.from_int(-3), &ring.rounded_div(ring.from_int(-8), &ring.from_int(3)));
    assert_el_eq!(&ring, &ring.from_int(-3), &ring.rounded_div(ring.from_int(8), &ring.from_int(-3)));
    assert_el_eq!(&ring, &ring.from_int(3), &ring.rounded_div(ring.from_int(-8), &ring.from_int(-3)));

    assert_el_eq!(&ring, &ring.from_int(4), &ring.rounded_div(ring.from_int(7), &ring.from_int(2)));
    assert_el_eq!(&ring, &ring.from_int(-4), &ring.rounded_div(ring.from_int(-7), &ring.from_int(2)));
    assert_el_eq!(&ring, &ring.from_int(-4), &ring.rounded_div(ring.from_int(7), &ring.from_int(-2)));
    assert_el_eq!(&ring, &ring.from_int(4), &ring.rounded_div(ring.from_int(-7), &ring.from_int(-2)));
}

#[test]
fn test_int_div_assumption() {
    assert_eq!(-1, -10 / 8);
    assert_eq!(-1, 10 / -8);
    assert_eq!(1, 10 / 8);
    assert_eq!(1, -10 / -8);
}

#[test]
fn test_rounded_div() {
    let ZZ = StaticRing::<i32>::RING;
    assert_el_eq!(&ZZ, &3, &ZZ.rounded_div(20, &7));
    assert_el_eq!(&ZZ, &-3, &ZZ.rounded_div(-20, &7));
    assert_el_eq!(&ZZ, &-3, &ZZ.rounded_div(20, &-7));
    assert_el_eq!(&ZZ, &3, &ZZ.rounded_div(-20, &-7));
    assert_el_eq!(&ZZ, &3, &ZZ.rounded_div(22, &7));
    assert_el_eq!(&ZZ, &-3, &ZZ.rounded_div(-22, &7));
    assert_el_eq!(&ZZ, &-3, &ZZ.rounded_div(22, &-7));
    assert_el_eq!(&ZZ, &3, &ZZ.rounded_div(-22, &-7));
}