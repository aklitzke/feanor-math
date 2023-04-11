use crate::delegate::DelegateRing;
use crate::divisibility::DivisibilityRing;
use crate::divisibility::DivisibilityRingStore;
use crate::euclidean::EuclideanRing;
use crate::euclidean::EuclideanRingStore;
use crate::field::Field;
use crate::integer::IntegerRing;
use crate::integer::IntegerRingStore;
use crate::ordered::OrderedRingStore;
use crate::ring::*;
use crate::rings::zn::*;
use crate::algorithms;

use std::cmp::Ordering;

#[derive(Clone)]
pub struct ZnBase<I: IntegerRingStore> {
    integer_ring: I,
    modulus: El<I>,
    inverse_modulus: El<I>,
    inverse_modulus_bitshift: usize,
}

pub type Zn<I> = RingValue<ZnBase<I>>;

impl<I: IntegerRingStore> Zn<I> {

    pub fn new(integer_ring: I, modulus: El<I>) -> Self {
        RingValue::from(ZnBase::new(integer_ring, modulus))
    }
}

impl<I: IntegerRingStore> ZnBase<I> {

    pub fn new(integer_ring: I, modulus: El<I>) -> Self {
        assert!(integer_ring.is_geq(&modulus, &integer_ring.from_z(2)));
        // have k such that 2^k > modulus^2
        // then (2^k / modulus) * x >> k differs at most 1 from floor(x / modulus)
        // if x < n^2, which is the case after multiplication
        let k = integer_ring.abs_highest_set_bit(&modulus).unwrap() * 2 + 2;
        let inverse_modulus = integer_ring.euclidean_div(
            integer_ring.pow(&integer_ring.from_z(2), k as usize), 
            &modulus
        );
        return ZnBase {
            integer_ring: integer_ring,
            modulus: modulus,
            inverse_modulus: inverse_modulus,
            inverse_modulus_bitshift: k
        };
    }

    fn project_leq_n_square(&self, n: &mut El<I>) {
        assert!(self.integer_ring.cmp(&n, &self.integer_ring.zero()) != Ordering::Less);
        let mut subtract = self.integer_ring.mul_ref(&n, &self.inverse_modulus);
        self.integer_ring.euclidean_div_pow_2(&mut subtract, self.inverse_modulus_bitshift);
        self.integer_ring.mul_assign_ref(&mut subtract, &self.modulus);
        self.integer_ring.sub_assign(n, subtract);
        if self.integer_ring.is_geq(&n, &self.modulus) {
            self.integer_ring.sub_assign_ref(n, &self.modulus);
        }
        assert!(self.integer_ring.is_lt(&n, &self.modulus), "The input is not smaller than {}^2", self.integer_ring.format(&self.modulus));
    }

    pub fn project(&self, n: El<I>) -> <Self as RingBase>::Element {
        let mut red_n = n;
        let negated = self.integer_ring.is_neg(&red_n);
        if negated {
            self.integer_ring.negate_inplace(&mut red_n);
        }
        if self.integer_ring.is_lt(&red_n, &self.modulus) {
            // already in the interval [0, self.modulus[
        } else if self.integer_ring.abs_highest_set_bit(&red_n).unwrap_or(0) + 1 < self.integer_ring.abs_highest_set_bit(&self.modulus).unwrap() * 2 {
            self.project_leq_n_square(&mut red_n);
        } else {
            red_n = self.integer_ring.euclidean_rem(red_n, &self.modulus);
        };
        debug_assert!(self.integer_ring.is_lt(&red_n, &self.modulus));
        let result = ZnEl(red_n);
        if negated {
            return self.negate(result);
        } else {
            return result;
        }
    }

    ///
    /// Returns either the inverse of x (as Ok()) or a nontrivial 
    /// factor of the modulus (as Err())
    /// 
    pub fn invert(&self, x: ZnEl<I>) -> Result<ZnEl<I>, El<I>> {
        let (s, _, d) = algorithms::eea::eea(x.0.clone(), self.modulus.clone(), &self.integer_ring);
        if self.integer_ring.is_neg_one(&d) || self.integer_ring.is_one(&d) {
            Ok(self.project(s))
        } else {
            Err(d)
        }
    }

    pub fn is_field(self) -> Result<FpBase<I>, ZnBase<I>> 
        where I: HashableElRingStore
    {
        if algorithms::miller_rabin::is_prime(self.integer_ring(), &self.modulus, 6) {
            Ok(FpBase { base: self })
        } else {
            Err(self)
        }
    }
}

pub struct ZnEl<I: IntegerRingStore>(El<I>);

impl<I: IntegerRingStore> Clone for ZnEl<I> {

    fn clone(&self) -> Self {
        ZnEl(self.0.clone())
    }
}

impl<I: IntegerRingStore> Copy for ZnEl<I> 
    where El<I>: Copy
{}

impl<I: IntegerRingStore> RingBase for ZnBase<I> {

    type Element = ZnEl<I>;

    fn add_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) {
        self.integer_ring.add_assign_ref(&mut lhs.0, &rhs.0);
        if self.integer_ring.is_geq(&lhs.0, &self.modulus) {
            self.integer_ring.sub_assign_ref(&mut lhs.0, &self.modulus);
        }
    }

    fn add_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) {
        self.integer_ring.add_assign(&mut lhs.0, rhs.0);
        if self.integer_ring.is_geq(&lhs.0, &self.modulus) {
            self.integer_ring.sub_assign_ref(&mut lhs.0, &self.modulus);
        }
    }

    fn sub_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) {
        self.integer_ring.sub_assign_ref(&mut lhs.0, &rhs.0);
        if self.integer_ring.is_neg(&lhs.0) {
            self.integer_ring.add_assign_ref(&mut lhs.0, &self.modulus);
        }
    }

    fn negate_inplace(&self, lhs: &mut Self::Element) {
        if !self.integer_ring.is_zero(&lhs.0) {
            self.integer_ring.negate_inplace(&mut lhs.0);
            self.integer_ring.add_assign_ref(&mut lhs.0, &self.modulus);
        }
    }

    fn mul_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) {
        self.integer_ring.mul_assign(&mut lhs.0, rhs.0);
        self.project_leq_n_square(&mut lhs.0);
    }

    fn mul_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) {
        self.integer_ring.mul_assign_ref(&mut lhs.0, &rhs.0);
        self.project_leq_n_square(&mut lhs.0);
    }

    fn from_z(&self, value: i32) -> Self::Element {
        self.project(self.integer_ring.from_z(value))
    }

    fn eq(&self, lhs: &Self::Element, rhs: &Self::Element) -> bool {
        self.integer_ring.eq(&lhs.0, &rhs.0)
    }

    fn is_zero(&self, value: &Self::Element) -> bool {
        self.integer_ring.is_zero(&value.0)
    }

    fn is_one(&self, value: &Self::Element) -> bool {
        self.integer_ring.is_one(&value.0)
    }
    
    fn is_commutative(&self) -> bool { true }
    fn is_noetherian(&self) -> bool { true }

    fn dbg<'a>(&self, value: &Self::Element, out: &mut std::fmt::Formatter<'a>) -> std::fmt::Result {
        self.integer_ring.get_ring().dbg(&value.0, out)
    }

}

impl<I: IntegerRingStore> DivisibilityRing for ZnBase<I> {
    
    fn checked_left_div(&self, lhs: &Self::Element, rhs: &Self::Element) -> Option<Self::Element> {
        let d = algorithms::eea::gcd(lhs.0.clone(), rhs.0.clone(), &self.integer_ring);
        if let Ok(inv) = self.invert(self.project(self.integer_ring.checked_div(&rhs.0, &d).unwrap())) {
            return Some(self.mul(inv, self.project(self.integer_ring.checked_div(&lhs.0, &d).unwrap())));
        } else {
            return None;
        }
    }
}

impl<I: IntegerRingStore, J: IntegerRingStore> CanonicalHom<ZnBase<J>> for ZnBase<I> {

    type Homomorphism =  <I::Type as CanonicalHom<J::Type>>::Homomorphism;

    fn has_canonical_hom(&self, from: &ZnBase<J>) -> Option<Self::Homomorphism> {
        let base_hom = <I::Type as CanonicalHom<J::Type>>::has_canonical_hom(self.integer_ring.get_ring(), from.integer_ring.get_ring())?;
        if self.integer_ring.eq(
            &self.modulus, 
            &<I::Type as CanonicalHom<J::Type>>::map_in(self.integer_ring.get_ring(), from.integer_ring.get_ring(), from.modulus.clone(), &base_hom)
        ) {
            Some(base_hom)
        } else {
            None
        }
    }

    fn map_in(&self, from: &ZnBase<J>, el: <ZnBase<J> as RingBase>::Element, hom: &Self::Homomorphism) -> Self::Element {
        ZnEl(<I::Type as CanonicalHom<J::Type>>::map_in(self.integer_ring.get_ring(), from.integer_ring.get_ring(), el.0, hom))
    }
}

impl<I: IntegerRingStore, J: IntegerRingStore> CanonicalIso<ZnBase<J>> for ZnBase<I> {

    type Isomorphism = <I::Type as CanonicalIso<J::Type>>::Isomorphism;

    fn has_canonical_iso(&self, from: &ZnBase<J>) -> Option<Self::Isomorphism> {
        let base_iso = <I::Type as CanonicalIso<J::Type>>::has_canonical_iso(self.integer_ring.get_ring(), from.integer_ring.get_ring())?;
        if self.integer_ring.eq(
             &self.modulus, 
            &<I::Type as CanonicalHom<J::Type>>::map_in(self.integer_ring.get_ring(), from.integer_ring.get_ring(), from.modulus.clone(), &base_iso)
        ) {
            Some(base_iso)
        } else {
            None
        }
    }

    fn map_out(&self, from: &ZnBase<J>, el: Self::Element, iso: &Self::Isomorphism) -> <ZnBase<J> as RingBase>::Element {
        ZnEl(<I::Type as CanonicalIso<J::Type>>::map_out(self.integer_ring.get_ring(), from.integer_ring.get_ring(), el.0, iso))
    }
}

impl<I: IntegerRingStore, J: IntegerRing + ?Sized> CanonicalHom<J> for ZnBase<I> 
    where J: CanonicalIso<J>
{
    type Homomorphism = <I::Type as CanonicalHom<J>>::Homomorphism;

    fn has_canonical_hom(&self, from: &J) -> Option<Self::Homomorphism> {
        <I::Type as CanonicalHom<J>>::has_canonical_hom(self.integer_ring().get_ring(), from)
    }

    fn map_in(&self, from: &J, el: J::Element, hom: &Self::Homomorphism) -> Self::Element {
        self.project(<I::Type as CanonicalHom<J>>::map_in(self.integer_ring().get_ring(), from, el, hom))
    }
}

pub struct ZnBaseElementsIter<'a, I>
    where I: IntegerRingStore
{
    ring: &'a ZnBase<I>,
    current: El<I>
}

impl<'a, I> Iterator for ZnBaseElementsIter<'a, I>
    where I: IntegerRingStore
{
    type Item = ZnEl<I>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ring.integer_ring().is_lt(&self.current, self.ring.modulus()) {
            let result = self.current.clone();
            self.ring.integer_ring().add_assign(&mut self.current, self.ring.integer_ring().one());
            return Some(ZnEl(result));
        } else {
            return None;
        }
    }
}

impl<I: IntegerRingStore> ZnRing for ZnBase<I> {
    
    type IntegerRingBase = I::Type;
    type Integers = I;
    type ElementsIter<'a> = ZnBaseElementsIter<'a, I>
        where Self: 'a;

    fn integer_ring(&self) -> &Self::Integers {
        &self.integer_ring
    }

    fn modulus(&self) -> &El<Self::Integers> {
        &self.modulus
    }

    fn smallest_positive_lift(&self, el: Self::Element) -> El<Self::Integers> {
        el.0
    }

    fn elements<'a>(&'a self) -> ZnBaseElementsIter<'a, I> {
        ZnBaseElementsIter {
            ring: self,
            current: self.integer_ring().zero()
        }
    }

}

#[derive(Clone)]
pub struct FpBase<I: IntegerRingStore> {
    base: ZnBase<I>
}

pub type Fp<I> = RingValue<FpBase<I>>;

pub struct FpEl<I: IntegerRingStore>(ZnEl<I>);

impl<I: IntegerRingStore> Clone for FpEl<I> {

    fn clone(&self) -> Self {
        FpEl(self.0.clone())
    }
}

impl<I: IntegerRingStore> Copy for FpEl<I> 
    where El<I>: Copy
{}

impl<I: IntegerRingStore> FpBase<I> {

    pub fn get_base(&self) -> &ZnBase<I> {
        &self.base
    }
}

impl<I: IntegerRingStore> Fp<I> {

    pub fn new(integer_ring: I, prime: El<I>) -> Self {
        Self::from(FpBase::new(integer_ring, prime))
    }
}

impl<I: IntegerRingStore> FpBase<I> {

    pub fn new(integer_ring: I, prime: El<I>) -> Self {
        ZnBase::new(integer_ring, prime).is_field().ok().expect("not a prime")
    }
}

impl<I: IntegerRingStore> DelegateRing for FpBase<I> {

    type Element = FpEl<I>;
    type Base = ZnBase<I>;

    fn get_delegate(&self) -> &Self::Base {
        &self.base
    }

    fn delegate(&self, el: Self::Element) -> <Self::Base as RingBase>::Element {
        el.0
    }

    fn delegate_mut<'a>(&self, el: &'a mut Self::Element) -> &'a mut <Self::Base as RingBase>::Element {
        &mut el.0
    }

    fn delegate_ref<'a>(&self, el: &'a Self::Element) -> &'a <Self::Base as RingBase>::Element {
        &el.0
    }

    fn rev_delegate(&self, el: <Self::Base as RingBase>::Element) -> Self::Element {
        FpEl(el)
    }
}

impl<I: IntegerRingStore, J: IntegerRingStore> CanonicalHom<FpBase<J>> for FpBase<I> {
    
    type Homomorphism = <ZnBase<I> as CanonicalHom<ZnBase<J>>>::Homomorphism;

    fn has_canonical_hom(&self, from: &FpBase<J>) -> Option<Self::Homomorphism> {
        self.get_base().has_canonical_hom(from.get_base())
    }

    fn map_in(&self, from: &FpBase<J>, el: FpEl<J>, hom: &Self::Homomorphism) -> Self::Element {
        FpEl(<ZnBase<I> as CanonicalHom<ZnBase<J>>>::map_in(self.get_base(), from.get_base(), el.0, hom))
    }
}

impl<I: IntegerRingStore, J: IntegerRingStore> CanonicalIso<FpBase<J>> for FpBase<I> {

    type Isomorphism = <ZnBase<I> as CanonicalIso<ZnBase<J>>>::Isomorphism;

    fn has_canonical_iso(&self, from: &FpBase<J>) -> Option<Self::Isomorphism> {
        <ZnBase<I> as CanonicalIso<ZnBase<J>>>::has_canonical_iso(self.get_base(), from.get_base())
    }

    fn map_out(&self, from: &FpBase<J>, el: Self::Element, iso: &Self::Isomorphism) -> FpEl<J> {
        FpEl(<ZnBase<I> as CanonicalIso<ZnBase<J>>>::map_out(self.get_base(), from.get_base(), el.0, iso))
    }
}

impl<I: IntegerRingStore, J: IntegerRingStore> CanonicalHom<ZnBase<J>> for FpBase<I> {

    type Homomorphism = <ZnBase<I> as CanonicalHom<ZnBase<J>>>::Homomorphism;

    fn has_canonical_hom(&self, from: &ZnBase<J>) -> Option<Self::Homomorphism> {
        self.get_base().has_canonical_hom(from)
    }

    fn map_in(&self, from: &ZnBase<J>, el: ZnEl<J>, hom: &Self::Homomorphism) -> Self::Element {
        FpEl(<ZnBase<I> as CanonicalHom<ZnBase<J>>>::map_in(self.get_base(), from, el, hom))
    }
}

impl<I: IntegerRingStore, J: IntegerRingStore> CanonicalIso<ZnBase<J>> for FpBase<I> {

    type Isomorphism = <ZnBase<I> as CanonicalIso<ZnBase<J>>>::Isomorphism;

    fn has_canonical_iso(&self, from: &ZnBase<J>) -> Option<Self::Isomorphism> {
        <ZnBase<I> as CanonicalIso<ZnBase<J>>>::has_canonical_iso(self.get_base(), from)
    }

    fn map_out(&self, from: &ZnBase<J>, el: Self::Element, iso: &Self::Isomorphism) -> <ZnBase<J> as RingBase>::Element {
        <ZnBase<I> as CanonicalIso<ZnBase<J>>>::map_out(self.get_base(), from, el.0, iso)
    }
}

impl<I: IntegerRingStore, J: IntegerRing + ?Sized> CanonicalHom<J> for FpBase<I> 
    where J: CanonicalIso<J>
{
    type Homomorphism = <ZnBase<I> as CanonicalHom<J>>::Homomorphism;

    fn has_canonical_hom(&self, from: &J) -> Option<Self::Homomorphism> {
        self.get_base().has_canonical_hom(from)
    }

    fn map_in(&self, from: &J, el: J::Element, hom: &Self::Homomorphism) -> Self::Element {
        FpEl(<ZnBase<I> as CanonicalHom<J>>::map_in(self.get_base(), from, el, hom))
    }
}

impl<I: IntegerRingStore, J: IntegerRingStore, K: IntegerRingStore> CanonicalHom<zn_rns::ZnBase<J, K>> for FpBase<I> {
    
    type Homomorphism = (usize, <FpBase<I> as CanonicalHom<FpBase<J>>>::Homomorphism);

    fn has_canonical_hom(&self, from: &zn_rns::ZnBase<J, K>) -> Option<Self::Homomorphism> {
        for i in 0..from.prime_component_count() {
            if let Some(hom) = self.has_canonical_hom(from.prime_component(i).get_ring()) {
                return Some((i, hom));
            }
        }
        return None;
    }

    fn map_in(&self, from: &zn_rns::ZnBase<J, K>, el: zn_rns::ZnEl<J>, (index, hom): &Self::Homomorphism) -> Self::Element {
        <Self as CanonicalHom<FpBase<J>>>::map_in(self, from.prime_component(*index).get_ring(), from.mod_prime_component(*index, &el).clone(), hom)
    }

    fn map_in_ref(&self, from: &zn_rns::ZnBase<J, K>, el: &zn_rns::ZnEl<J>, (index, hom): &Self::Homomorphism) -> Self::Element {
        <Self as CanonicalHom<FpBase<J>>>::map_in(self, from.prime_component(*index).get_ring(), from.mod_prime_component(*index, el).clone(), hom)
    }
}

impl<I: IntegerRingStore> DivisibilityRing for FpBase<I> {

    fn checked_left_div(&self, lhs: &Self::Element, rhs: &Self::Element) -> Option<Self::Element> {
        if self.is_zero(rhs) {
            None
        } else {
            Some(self.mul_ref_fst(
                lhs, 
                FpEl(self.get_base().invert(rhs.0.clone()).ok().unwrap())
            ))
        }
    }
}

impl<I: IntegerRingStore> EuclideanRing for FpBase<I> {

    fn euclidean_div_rem(&self, lhs: Self::Element, rhs: &Self::Element) -> (Self::Element, Self::Element) {
        assert!(!self.is_zero(rhs));
        (self.checked_left_div(&lhs, rhs).unwrap(), self.zero())
    }

    fn euclidean_deg(&self, val: &Self::Element) -> Option<usize> {
        if self.is_zero(val) {
            Some(0)
        } else {
            Some(1)
        }
    }

    fn euclidean_rem(&self, _: Self::Element, rhs: &Self::Element) -> Self::Element {
        assert!(!self.is_zero(rhs));
        self.zero()
    }
}

impl<I: IntegerRingStore> Field for FpBase<I> {}

pub struct FpBaseElementsIter<'a, I>
    where I: IntegerRingStore
{
    iter: ZnBaseElementsIter<'a, I>
}

impl<'a, I> Iterator for FpBaseElementsIter<'a, I>
    where I: IntegerRingStore
{
    type Item = FpEl<I>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(FpEl)
    }
}

impl<I: IntegerRingStore> ZnRing for FpBase<I> {
    
    type IntegerRingBase = I::Type;
    type Integers = I;
    type ElementsIter<'a> = FpBaseElementsIter<'a, I>
        where Self: 'a;

    fn integer_ring(&self) -> &Self::Integers {
        self.get_base().integer_ring()
    }

    fn modulus(&self) -> &El<Self::Integers> {
        self.get_base().modulus()
    }

    fn smallest_positive_lift(&self, el: Self::Element) -> El<Self::Integers> {
        self.get_base().smallest_positive_lift(el.0)
    }

    fn elements<'a>(&'a self) -> FpBaseElementsIter<'a, I> {
        FpBaseElementsIter {
            iter: self.get_base().elements()
        }
    }
}

#[cfg(test)]
use crate::primitive_int::*;
#[cfg(test)]
use crate::rings::bigint::*;

#[test]
fn test_mul() {
    const ZZ: RingValue<DefaultBigIntRing> = DefaultBigIntRing::RING;
    let Z257 = ZnBase::new(ZZ, ZZ.from_z(257));
    let x = Z257.project(ZZ.from_z(256));
    assert!(Z257.eq(&Z257.one(), &Z257.mul_ref(&x, &x)));
}

#[test]
fn test_project() {
    const ZZ: StaticRing<i64> = StaticRing::RING;
    let Z17 = Zn::new(ZZ, 17);
    for k in 0..289 {
        assert!(Z17.eq(&Z17.from_z((289 - k) % 17), &Z17.get_ring().project(-k as i64)));
    }
}

#[test]
fn test_ring_axioms_znbase() {
    let ZZ = Zn::new(StaticRing::<i64>::RING, 63);
    test_ring_axioms(&ZZ, [0, 1, 3, 7, 9, 62, 8, 10, 11, 12].iter().cloned().map(|x| ZZ.from_z(x)))
}

#[test]
fn test_zn_ring_axioms_znbase() {
    test_zn_ring_axioms(Zn::new(StaticRing::<i64>::RING, 17));
    test_zn_ring_axioms(Zn::new(StaticRing::<i64>::RING, 63));
}