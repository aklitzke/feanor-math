use crate::pid::PrincipalIdealRing;
use crate::primitive_int::StaticRing;
use crate::ring::*;
use crate::divisibility::DivisibilityRing;
use crate::algorithms;
use crate::integer::*;
use crate::homomorphism::*;
use crate::ordered::*;
use super::field::AsFieldBase;
use super::finite::FiniteRing;
use crate::rings::finite::FiniteRingStore;

///
/// This module contains [`zn_barett::Zn`], a general-purpose implementation of
/// Barett reduction. It is relatively slow when instantiated with small fixed-size
/// integer type. 
/// 
pub mod zn_barett;
///
/// This module contains [`zn_42::Zn`], a heavily optimized implementation of `Z/nZ`
/// for moduli `n` with at most 41 bits. Note that for most purposes, this should be
/// replace by the new module [`zn_64::Zn`].
/// 
pub mod zn_42;
///
/// This module contains [`zn_64::Zn`], the new, heavily optimized implementation of `Z/nZ`
/// for moduli `n` of size slightly smaller than 64 bits.
/// 
pub mod zn_64;
///
/// This module contains [`zn_static::Zn`], an implementation of `Z/nZ` for a small `n`
/// that is known at compile-time.
/// 
pub mod zn_static;
///
/// This module contains [`zn_rns::Zn`], a residue number system (RNS) implementation of
/// `Z/nZ` for highly composite `n`. 
/// 
pub mod zn_rns;

///
/// Trait for all rings that represent a quotient of the integers `Z/nZ` for some integer `n`.
/// 
pub trait ZnRing: PrincipalIdealRing + FiniteRing + CanHomFrom<Self::IntegerRingBase> {

    /// 
    /// there seems to be a problem with associated type bounds, hence we cannot use `Integers: IntegerRingStore`
    /// or `Integers: RingStore<Type: IntegerRing>`
    /// 
    type IntegerRingBase: IntegerRing + ?Sized;
    type Integers: RingStore<Type = Self::IntegerRingBase>;

    fn integer_ring(&self) -> &Self::Integers;
    fn modulus(&self) -> &El<Self::Integers>;

    ///
    /// Computes the smallest positive lift for some `x` in `Z/nZ`, i.e. the smallest positive integer `m` such that
    /// `m = x mod n`.
    /// 
    /// This will be one of `0, 1, ..., n - 1`. If an integer in `-(n - 1)/2, ..., -1, 0, 1, ..., (n - 1)/2` (for odd `n`)
    /// is needed instead, use [`ZnRing::smallest_lift()`].
    /// 
    fn smallest_positive_lift(&self, el: Self::Element) -> El<Self::Integers>;

    ///
    /// Computes the smallest lift for some `x` in `Z/nZ`, i.e. the smallest integer `m` such that
    /// `m = x mod n`.
    /// 
    /// This will be one of `-(n - 1)/2, ..., -1, 0, 1, ..., (n - 1)/2` (for odd `n`). If an integer in `0, 1, ..., n - 1`
    /// is needed instead, use [`ZnRing::smallest_positive_lift()`].
    /// 
    fn smallest_lift(&self, el: Self::Element) -> El<Self::Integers> {
        let result = self.smallest_positive_lift(el);
        let mut mod_half = self.integer_ring().clone_el(self.modulus());
        self.integer_ring().euclidean_div_pow_2(&mut mod_half, 1);
        if self.integer_ring().is_gt(&result, &mod_half) {
            return self.integer_ring().sub_ref_snd(result, self.modulus());
        } else {
            return result;
        }
    }

    ///
    /// Returns whether this ring is a field, i.e. whether `n` is prime.
    /// 
    fn is_field(&self) -> bool {
        algorithms::miller_rabin::is_prime_base(RingRef::new(self), 10)
    }
}

pub mod generic_impls {
    use std::marker::PhantomData;

    use crate::ring::*;
    use crate::divisibility::DivisibilityRingStore;
    use crate::integer::{IntegerRing, IntegerRingStore};
    use crate::algorithms;
    use super::{ZnRing, ZnRingStore};
    use crate::homomorphism::*;

    #[allow(type_alias_bounds)]
    pub type Homomorphism<R: ZnRing, S: ZnRing> = (<S as CanHomFrom<S::IntegerRingBase>>::Homomorphism, <S::IntegerRingBase as CanHomFrom<R::IntegerRingBase>>::Homomorphism);

    pub fn has_canonical_hom<R: ZnRing, S: ZnRing>(from: &R, to: &S) -> Option<Homomorphism<R, S>> 
        where S::IntegerRingBase: CanHomFrom<R::IntegerRingBase>
    {
        let hom = <S::IntegerRingBase as CanHomFrom<R::IntegerRingBase>>::has_canonical_hom(to.integer_ring().get_ring(), from.integer_ring().get_ring())?;
        if to.integer_ring().checked_div(&<S::IntegerRingBase as CanHomFrom<R::IntegerRingBase>>::map_in_ref(&to.integer_ring().get_ring(), from.integer_ring().get_ring(), from.modulus(), &hom), &to.modulus()).is_some() {
            Some((to.has_canonical_hom(to.integer_ring().get_ring()).unwrap(), hom))
        } else {
            None
        }
    }

    pub fn map_in<R: ZnRing, S: ZnRing>(from: &R, to: &S, el: R::Element, hom: &Homomorphism<R, S>) -> S::Element 
        where S::IntegerRingBase: CanHomFrom<R::IntegerRingBase>
    {
        to.map_in(to.integer_ring().get_ring(), <S::IntegerRingBase as CanHomFrom<R::IntegerRingBase>>::map_in(to.integer_ring().get_ring(), from.integer_ring().get_ring(), from.smallest_positive_lift(el), &hom.1), &hom.0)
    }

    pub struct IntegerToZnHom<I: ?Sized + IntegerRing, J: ?Sized + IntegerRing, R: ?Sized + ZnRing>
        where I: CanonicalIso<R::IntegerRingBase> + CanonicalIso<J>
    {
        highbit_mod: usize,
        highbit_bound: usize,
        int_ring: PhantomData<I>,
        to_large_int_ring: PhantomData<J>,
        hom: <I as CanHomFrom<R::IntegerRingBase>>::Homomorphism,
        iso: <I as CanonicalIso<R::IntegerRingBase>>::Isomorphism,
        iso2: <I as CanonicalIso<J>>::Isomorphism
    }

    ///
    /// See [`map_in_from_int()`].
    /// This will only ever return `None` if one of the integer ring `has_canonical_hom/iso` returns `None`.
    /// 
    pub fn has_canonical_hom_from_int<I: ?Sized + IntegerRing, J: ?Sized + IntegerRing, R: ?Sized + ZnRing>(from: &I, to: &R, to_large_int_ring: &J, bounded_reduce_bound: Option<&J::Element>) -> Option<IntegerToZnHom<I, J, R>>
        where I: CanonicalIso<R::IntegerRingBase> + CanonicalIso<J>
    {
        if let Some(bound) = bounded_reduce_bound {
            Some(IntegerToZnHom {
                highbit_mod: to.integer_ring().abs_highest_set_bit(to.modulus()).unwrap(),
                highbit_bound: to_large_int_ring.abs_highest_set_bit(bound).unwrap(),
                int_ring: PhantomData,
                to_large_int_ring: PhantomData,
                hom: from.has_canonical_hom(to.integer_ring().get_ring())?,
                iso: from.has_canonical_iso(to.integer_ring().get_ring())?,
                iso2: from.has_canonical_iso(to_large_int_ring)?
            })
        } else {
            Some(IntegerToZnHom {
                highbit_mod: to.integer_ring().abs_highest_set_bit(to.modulus()).unwrap(),
                highbit_bound: usize::MAX,
                int_ring: PhantomData,
                to_large_int_ring: PhantomData,
                hom: from.has_canonical_hom(to.integer_ring().get_ring())?,
                iso: from.has_canonical_iso(to.integer_ring().get_ring())?,
                iso2: from.has_canonical_iso(to_large_int_ring)?
            })
        }
    }

    ///
    /// A parameterized, generic variant of the reduction `Z -> Z/nZ`.
    /// It considers the following situations:
    ///  - the source ring `Z` might not be large enough to represent `n`
    ///  - the integer ring associated to the destination ring `Z/nZ` might not be large enough to represent the input
    ///  - the destination ring might use Barett reductions (or similar) for fast modular reduction if the input is bounded by some fixed bound `B`
    ///  - general modular reduction modulo `n` is only performed in the source ring if necessary
    /// 
    /// In particular, we use the following additional parameters:
    ///  - `to_large_int_ring`: an integer ring that can represent all integers for which we can perform fast modular reduction (i.e. those bounded by `B`)
    ///  - `from_positive_representative_exact`: a function that performs the restricted reduction `{0, ..., n - 1} -> Z/nZ`
    ///  - `from_positive_representative_bounded`: a function that performs the restricted reduction `{0, ..., B - 1} -> Z/nZ`
    /// 
    /// Note that the input size estimates consider only the bitlength of numbers, and so there is a small margin in which a reduction method for larger
    /// numbers than necessary is used. Furthermore, if the integer rings used can represent some but not all positive numbers of a certain bitlength, 
    /// there might be rare edge cases with panics/overflows. 
    /// 
    /// In particular, if the input integer ring `Z` can represent the input `x`, but not `n` AND `x` and `n` have the same bitlength, this function might
    /// decide that we have to perform generic modular reduction (even though `x < n`), and try to map `n` into `Z`. This is never a problem if the primitive
    /// integer rings `StaticRing::<ixx>::RING` are used, or if `B >= 2n`.
    /// 
    pub fn map_in_from_int<I: ?Sized + IntegerRing, J: ?Sized + IntegerRing, R: ?Sized + ZnRing, F, G>(from: &I, to: &R, to_large_int_ring: &J, el: I::Element, hom: &IntegerToZnHom<I, J, R>, from_positive_representative_exact: F, from_positive_representative_bounded: G) -> R::Element
        where I: CanonicalIso<R::IntegerRingBase> + CanonicalIso<J>,
            F: FnOnce(El<R::Integers>) -> R::Element,
            G: FnOnce(J::Element) -> R::Element
    {
        let (neg, n) = if from.is_neg(&el) {
            (true, from.negate(el))
        } else {
            (false, el)
        };
        let ZZ = to.integer_ring().get_ring();
        let highbit_el = from.abs_highest_set_bit(&n).unwrap_or(0);

        let reduced = if highbit_el < hom.highbit_mod {
            from_positive_representative_exact(from.map_out(ZZ, n, &hom.iso))
        } else if highbit_el < hom.highbit_bound {
            from_positive_representative_bounded(from.map_out(to_large_int_ring, n, &hom.iso2))
        } else {
            from_positive_representative_exact(from.map_out(ZZ, from.euclidean_rem(n, &from.map_in_ref(ZZ, to.modulus(), &hom.hom)), &hom.iso))
        };
        if neg {
            to.negate(reduced)
        } else {
            reduced
        }
    }

    pub fn random_element<R: ZnRing, G: FnMut() -> u64>(ring: &R, rng: G) -> R::Element {
        ring.map_in(
            ring.integer_ring().get_ring(), 
            ring.integer_ring().get_uniformly_random(ring.modulus(), rng), 
            &ring.has_canonical_hom(ring.integer_ring().get_ring()).unwrap()
        )
    }

    pub fn checked_left_div<R: ZnRingStore>(ring: R, lhs: &El<R>, rhs: &El<R>, modulus: &El<<R::Type as ZnRing>::Integers>) -> Option<El<R>>
        where R::Type: ZnRing
    {
        if ring.is_zero(lhs) {
            return Some(ring.zero());
        }
        let int_ring = ring.integer_ring();
        let lhs_lift = ring.smallest_positive_lift(ring.clone_el(lhs));
        let rhs_lift = ring.smallest_positive_lift(ring.clone_el(rhs));
        let (s, _, d) = algorithms::eea::signed_eea(int_ring.clone_el(&rhs_lift), int_ring.clone_el(&modulus), int_ring);
        if let Some(quotient) = int_ring.checked_div(&lhs_lift, &d) {
            Some(ring.mul(ring.coerce(int_ring, quotient), ring.coerce(int_ring, s)))
        } else {
            None
        }
    }
}

///
/// The [`crate::ring::RingStore`] corresponding to [`ZnRing`].
/// 
pub trait ZnRingStore: FiniteRingStore
    where Self::Type: ZnRing
{    
    delegate!{ fn integer_ring(&self) -> &<Self::Type as ZnRing>::Integers }
    delegate!{ fn modulus(&self) -> &El<<Self::Type as ZnRing>::Integers> }
    delegate!{ fn smallest_positive_lift(&self, el: El<Self>) -> El<<Self::Type as ZnRing>::Integers> }
    delegate!{ fn smallest_lift(&self, el: El<Self>) -> El<<Self::Type as ZnRing>::Integers> }
    delegate!{ fn is_field(&self) -> bool }

    fn as_field(self) -> Result<RingValue<AsFieldBase<Self>>, Self> 
        where Self: Sized
    {
        if self.is_field() {
            Ok(RingValue::from(AsFieldBase::unsafe_create(self)))
        } else {
            Err(self)
        }
    }
}

impl<R: RingStore> ZnRingStore for R
    where R::Type: ZnRing
{}

///
/// Trait for algorithms that require some implementation of
/// `Z/nZ`, but do not care which.
/// 
/// If you want to avoid the boilerplate code to create such an
/// object, look at the experimental macro [`generate_zn_function`].
/// 
pub trait ZnOperation {
    
    fn call<R: ZnRingStore>(self, ring: R)
        where R::Type: ZnRing;
}

///
/// A helper macro to easily create an object implementing [`ZnOperation`].
/// This is experimental, and tries to make it easy to write code that requires
/// some finite field `Z/nZ`, but does not care about its implementation.
/// 
/// # Example
/// ```
/// # use feanor_math::ring::*;
/// # use feanor_math::homomorphism::*;
/// # use feanor_math::rings::zn::*;
/// # use feanor_math::generate_zn_function;
/// # use feanor_math::primitive_int::*;
/// # use feanor_math::integer::*;
/// # use feanor_math::assert_el_eq;
/// 
/// let int_value = 4;
/// // work in Z/17Z without explicitly choosing an implementation
/// choose_zn_impl(StaticRing::<i64>::RING, 17, generate_zn_function!(
///     < {'a} > [_: &'a i64 = &int_value] |Zn: R, (int_value, ): (&i64, )| {
///         let value = Zn.coerce(Zn.integer_ring(), int_cast(*int_value, Zn.integer_ring(), &StaticRing::<i64>::RING));
///         assert_el_eq!(&Zn, &Zn.int_hom().map(-1), &Zn.mul_ref(&value, &value));
///     }
/// ));
/// ```
/// 
/// # Warning
/// 
/// As type for the ring parameter, you can use `R` - I do not think Rust should allow this
/// (it violates macro hygenie), but let's be happy that it works, otherwise this would be
/// impossible.
/// 
#[macro_export]
macro_rules! generate_zn_function {
    (< $({$gen_param:tt $(: $($gen_constraint:tt)*)?}),* > $bindings:tt $lambda:expr) => {
        {
            struct LocalZnOperation<$($gen_param),*> 
                where $($($gen_param: $($gen_constraint)*,)?)*
            {
                args: $crate::generate_binding_type!{ $bindings }
            }

            impl<$($gen_param),*>  $crate::rings::zn::ZnOperation for LocalZnOperation<$($gen_param),*> 
                where $($($gen_param: $($gen_constraint)*,)?)*
            {
                
                fn call<R: $crate::rings::zn::ZnRingStore>(self, ring: R)
                    where <R as $crate::ring::RingStore>::Type: $crate::rings::zn::ZnRing
                {
                    ($lambda)(ring, self.args);
                } 
            }

            LocalZnOperation { args: $crate::generate_binding_value!($bindings) }
        }
    };
}

///
/// Calls the given function with some implementation of the ring
/// `Z/nZ`, chosen depending on `n` to provide best performance.
/// 
/// To avoid the boilerplate code that comes with manually implementing
/// [`ZnOperation`], consider using the experimental macro [`generate_zn_operation`].
/// 
/// # Example
/// ```
/// # use feanor_math::ring::*;
/// # use feanor_math::homomorphism::*;
/// # use feanor_math::rings::zn::*;
/// # use feanor_math::generate_zn_function;
/// # use feanor_math::primitive_int::*;
/// # use feanor_math::integer::*;
/// # use feanor_math::assert_el_eq;
/// 
/// let int_value = 4;
/// // work in Z/17Z without explicitly choosing an implementation
/// choose_zn_impl(StaticRing::<i64>::RING, 17, generate_zn_function!(
///     < {'a} > [_: &'a i64 = &int_value] |Zn: R, (int_value, ): (&i64, )| {
///         let value = Zn.coerce(Zn.integer_ring(), int_cast(*int_value, Zn.integer_ring(), &StaticRing::<i64>::RING));
///         assert_el_eq!(&Zn, &Zn.int_hom().map(-1), &Zn.mul_ref(&value, &value));
///     }
/// ));
/// ```
/// 
pub fn choose_zn_impl<I, F>(ZZ: I, n: El<I>, f: F)
    where I: IntegerRingStore,
        I::Type: IntegerRing,
        F: ZnOperation
{
    if ZZ.abs_highest_set_bit(&n).unwrap_or(0) < 57 {
        f.call(zn_64::Zn::new(StaticRing::<i64>::RING.coerce(&ZZ, n) as u64));
    } else {
        f.call(zn_barett::Zn::new(ZZ, n));
    }
}

#[cfg(any(test, feature = "generic_tests"))]
pub mod generic_tests {

    use super::*;
    use crate::primitive_int::{StaticRingBase, StaticRing};

    pub fn test_zn_axioms<R: ZnRingStore>(R: R)
        where R::Type: ZnRing,
            <R::Type as ZnRing>::IntegerRingBase: CanonicalIso<StaticRingBase<i128>> + CanonicalIso<StaticRingBase<i32>>
    {
        let ZZ = R.integer_ring();
        let n = R.modulus();

        assert!(R.is_zero(&R.coerce(ZZ, ZZ.clone_el(n))));
        assert!(R.is_field() == algorithms::miller_rabin::is_prime(ZZ, n, 10));

        let mut k = ZZ.one();
        while ZZ.is_lt(&k, &n) {
            assert!(!R.is_zero(&R.coerce(ZZ, ZZ.clone_el(&k))));
            ZZ.add_assign(&mut k, ZZ.one());
        }

        let all_elements = R.elements().collect::<Vec<_>>();
        assert_eq!(int_cast(ZZ.clone_el(n), &StaticRing::<i128>::RING, &ZZ) as usize, all_elements.len());
        for (i, x) in all_elements.iter().enumerate() {
            for (j, y) in all_elements.iter().enumerate() {
                assert!(i == j || !R.eq_el(x, y));
            }
        }
    }

    pub fn test_map_in_large_int<R: ZnRingStore>(R: R)
        where <R as RingStore>::Type: ZnRing + CanHomFrom<BigIntRingBase>
    {
        let ZZ_big = BigIntRing::RING;
        let n = ZZ_big.power_of_two(1000);
        let x = R.coerce(&ZZ_big, n);
        assert!(R.eq_el(&R.pow(R.int_hom().map(2), 1000), &x));
    }
}