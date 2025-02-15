use crate::algorithms;
use crate::divisibility::*;
use crate::pid::*;
use crate::field::Field;
use crate::mempool::GrowableMemoryProvider;
use crate::vector::VectorViewMut;
use crate::ring::*;
use crate::rings::poly::*;
use crate::vector::*;
use crate::vector::sparse::*;

use std::cmp::max;
use std::rc::Rc;

///
/// The univariate polynomial ring `R[X]`. Polynomials are stored as sparse vectors of
/// coefficients, thus giving improved performance in the case that most coefficients are
/// zero.
/// 
/// # Example
/// ```
/// # use feanor_math::ring::*;
/// # use feanor_math::homomorphism::*;
/// # use feanor_math::rings::poly::*;
/// # use feanor_math::rings::poly::sparse_poly::*;
/// # use feanor_math::primitive_int::*;
/// 
/// let ZZ = StaticRing::<i32>::RING;
/// let P = SparsePolyRing::new(ZZ, "X");
/// let x10_plus_1 = P.add(P.pow(P.indeterminate(), 10), P.int_hom().map(1));
/// let power = P.pow(x10_plus_1, 10);
/// assert_eq!(0, *P.coefficient_at(&power, 1));
/// ```
/// This ring has a [`CanonicalIso`] to [`dense_poly::DensePolyRingBase`].
/// ```
/// # use feanor_math::assert_el_eq;
/// # use feanor_math::homomorphism::*;
/// # use feanor_math::ring::*;
/// # use feanor_math::rings::poly::*;
/// # use feanor_math::rings::poly::dense_poly::*;
/// # use feanor_math::rings::poly::sparse_poly::*;
/// # use feanor_math::primitive_int::*;
/// 
/// let ZZ = StaticRing::<i32>::RING;
/// let P = SparsePolyRing::new(ZZ, "X");
/// let P2 = DensePolyRing::new(ZZ, "X");
/// let high_power_of_x = P.pow(P.indeterminate(), 10);
/// assert_el_eq!(&P2, &P2.pow(P2.indeterminate(), 10), &P.can_iso(&P2).unwrap().map(high_power_of_x));
/// ```
/// 
pub struct SparsePolyRingBase<R: RingStore> {
    base_ring: Rc<R>,
    unknown_name: &'static str,
    zero: El<R>
}

impl<R: RingStore + Clone> Clone for SparsePolyRingBase<R> {
    
    fn clone(&self) -> Self {
        SparsePolyRingBase {
            base_ring: self.base_ring.clone(), 
            unknown_name: self.unknown_name, 
            zero: self.base_ring.zero()
        }
    }
}

///
/// The univariate polynomial ring `R[X]`, with polynomials being stored as sparse vectors of coefficients.
/// For details, see [`SparsePolyRingBase`].
/// 
#[allow(type_alias_bounds)]
pub type SparsePolyRing<R: RingStore> = RingValue<SparsePolyRingBase<R>>;

impl<R: RingStore> SparsePolyRing<R> {

    pub fn new(base_ring: R, unknown_name: &'static str) -> Self {
        Self::from(SparsePolyRingBase::new(base_ring, unknown_name))
    }
}

impl<R: RingStore> SparsePolyRingBase<R> {

    pub fn new(base_ring: R, unknown_name: &'static str) -> Self {
        let zero = base_ring.zero();
        SparsePolyRingBase { 
            base_ring: Rc::new(base_ring), 
            unknown_name: unknown_name, 
            zero: zero
        }
    }

    fn degree_truncate(&self, el: &mut <Self as RingBase>::Element) {
        for i in (0..el.len()).rev() {
            if !self.base_ring.is_zero(&el.at(i)) {
                el.set_len(i + 1);
                return;
            }
        }
        el.set_len(0);
    }

    fn poly_div<F>(&self, lhs: &mut SparseVectorMut<Rc<R>>, rhs: &SparseVectorMut<Rc<R>>, mut left_div_lc: F) -> Option<SparseVectorMut<Rc<R>>>
        where F: FnMut(El<R>) -> Option<El<R>>
    {
        let lhs_val = std::mem::replace(lhs, self.zero());
        let (quo, rem) = algorithms::poly_div::sparse_poly_div(
            lhs_val, 
            rhs, 
            RingRef::new(self), 
            RingRef::new(self), 
            |x| left_div_lc(self.base_ring().clone_el(x)).ok_or(()),
            &self.base_ring().identity()
        ).ok()?;
        *lhs = rem;
        return Some(quo);
    }
}

impl<R: RingStore> RingBase for SparsePolyRingBase<R> {
    
    type Element = SparseVectorMut<Rc<R>>;

    fn clone_el(&self, val: &Self::Element) -> Self::Element {
        val.clone()
    }

    fn add_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) {
        lhs.set_len(max(lhs.len(), rhs.len()));
        for (i, c) in rhs.nontrivial_entries() {
            self.base_ring.add_assign_ref(lhs.at_mut(i), c);
        }
        self.degree_truncate(lhs);
    }

    fn add_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) {
        self.add_assign_ref(lhs, &rhs);
    }

    fn sub_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) {
        lhs.set_len(max(lhs.len(), rhs.len()));
        for (i, c) in rhs.nontrivial_entries() {
            self.base_ring.sub_assign_ref(lhs.at_mut(i), c);
        }
        self.degree_truncate(lhs);
    }

    fn negate_inplace(&self, lhs: &mut Self::Element) {
        lhs.scan(|_, c| self.base_ring.negate_inplace(c));
    }

    fn mul_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) {
        self.mul_assign_ref(lhs, &rhs);
    }

    fn mul_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) {
        *lhs = self.mul_ref(lhs, rhs);
    }

    fn zero(&self) -> Self::Element {
        SparseVectorMut::new(0, self.base_ring.clone())
    }
    
    fn from_int(&self, value: i32) -> Self::Element {
        let mut result = self.zero();
        result.set_len(1);
        *result.at_mut(0) = self.base_ring.int_hom().map(value);
        return result;
    }

    fn eq_el(&self, lhs: &Self::Element, rhs: &Self::Element) -> bool {
        if lhs.len() != rhs.len() {
            return false;
        }
        for (i, c) in lhs.nontrivial_entries() {
            if !self.base_ring.eq_el(rhs.at(i), c) {
                return false;
            }
        }
        for (i, c) in rhs.nontrivial_entries() {
            if !self.base_ring.eq_el(lhs.at(i), c) {
                return false;
            }
        }
        return true;
    }

    fn is_commutative(&self) -> bool {
        self.base_ring.is_commutative()
    }

    fn is_noetherian(&self) -> bool {
        // by Hilbert's basis theorem
        self.base_ring.is_noetherian()
    }

    fn dbg<'a>(&self, value: &Self::Element, out: &mut std::fmt::Formatter<'a>) -> std::fmt::Result {
        super::generic_impls::dbg_poly(self, value, out, self.unknown_name)
    }

    fn square(&self, value: &mut Self::Element) {
        *value = self.mul_ref(&value, &value);
    }

    fn mul_ref(&self, lhs: &Self::Element, rhs: &Self::Element) -> Self::Element {
        if lhs.len() == 0 || rhs.len() == 0 {
            return self.zero();
        }
        let mut result = SparseVectorMut::new(lhs.len() + rhs.len() - 1, self.base_ring.clone());
        for (i, c1) in lhs.nontrivial_entries() {
            for (j, c2) in rhs.nontrivial_entries() {
                self.base_ring.add_assign(result.at_mut(i + j), self.base_ring.mul_ref(c1, c2));
            }
        }
        // if the ring is not zero-divisor free
        self.degree_truncate(&mut result);
        return result;
    }

    fn mul_assign_int(&self, lhs: &mut Self::Element, rhs: i32) {
        if rhs == 0 {
            *lhs = self.zero();
        } else {
            lhs.scan(|_, c| self.base_ring.int_hom().mul_assign_map(c, rhs));
        }
    }
}

impl<R> PartialEq for SparsePolyRingBase<R> 
    where R: RingStore
{
    fn eq(&self, other: &Self) -> bool {
        self.base_ring.get_ring() == other.base_ring.get_ring()
    }
}

impl<R: RingStore> RingExtension for SparsePolyRingBase<R> {
    
    type BaseRing = R;

    fn base_ring<'a>(&'a self) -> &'a Self::BaseRing {
        &self.base_ring
    }

    fn from(&self, x: El<Self::BaseRing>) -> Self::Element {
        let mut result = self.zero();
        result.set_len(1);
        *result.at_mut(0) = x;
        return result;
    }
}

pub trait ImplGenericCanonicalIsoMarker: PolyRing {}

impl<R, M> ImplGenericCanonicalIsoMarker for dense_poly::DensePolyRingBase<R, M> 
    where R: RingStore, M: GrowableMemoryProvider<El<R>>
{}

impl<R, P> CanHomFrom<P> for SparsePolyRingBase<R> 
    where R: RingStore, R::Type: CanHomFrom<<P::BaseRing as RingStore>::Type>, P: ImplGenericCanonicalIsoMarker
{
    type Homomorphism = super::generic_impls::GenericCanHomFrom<P, Self>;

    fn has_canonical_hom(&self, from: &P) -> Option<Self::Homomorphism> {
        super::generic_impls::generic_has_canonical_hom(from, self)
    }

    fn map_in(&self, from: &P, el: P::Element, hom: &Self::Homomorphism) -> Self::Element {
        super::generic_impls::generic_map_in(from, self, el, hom)
    }
}

impl<R1, R2> CanHomFrom<SparsePolyRingBase<R1> > for SparsePolyRingBase<R2> 
    where R1: RingStore, R2: RingStore, R2::Type: CanHomFrom<R1::Type>
{
    type Homomorphism = <R2::Type as CanHomFrom<R1::Type>>::Homomorphism;

    fn has_canonical_hom(&self, from: &SparsePolyRingBase<R1>) -> Option<Self::Homomorphism> {
        self.base_ring().get_ring().has_canonical_hom(from.base_ring().get_ring())
    }

    fn map_in_ref(&self, from: &SparsePolyRingBase<R1> , el: &SparseVectorMut<Rc<R1>>, hom: &Self::Homomorphism) -> Self::Element {
        let mut result = SparseVectorMut::new(el.len(), self.base_ring.clone());
        for (j, c) in el.nontrivial_entries() {
            *result.at_mut(j) = self.base_ring().get_ring().map_in_ref(from.base_ring().get_ring(), c, hom);
        }
        return result;
    }

    fn map_in(&self, from: &SparsePolyRingBase<R1>, el: <SparsePolyRingBase<R1> as RingBase>::Element, hom: &Self::Homomorphism) -> Self::Element {
        self.map_in_ref(from, &el, hom)
    }
}

impl<R, P> CanonicalIso<P> for SparsePolyRingBase<R> 
    where R: RingStore, R::Type: CanonicalIso<<P::BaseRing as RingStore>::Type>, P: ImplGenericCanonicalIsoMarker
{
    type Isomorphism = super::generic_impls::GenericCanonicalIso<P, Self>;

    fn has_canonical_iso(&self, from: &P) -> Option<Self::Isomorphism> {
        self.base_ring().get_ring().has_canonical_iso(from.base_ring().get_ring())
    }

    fn map_out(&self, from: &P, el: Self::Element, iso: &Self::Isomorphism) -> P::Element {
        super::generic_impls::generic_map_out(from, self, el, iso)
    }
}

impl<R1, R2> CanonicalIso<SparsePolyRingBase<R1>> for SparsePolyRingBase<R2> 
    where R1: RingStore, R2: RingStore, R2::Type: CanonicalIso<R1::Type>
{
    type Isomorphism = <R2::Type as CanonicalIso<R1::Type>>::Isomorphism;

    fn has_canonical_iso(&self, from: &SparsePolyRingBase<R1>) -> Option<Self::Isomorphism> {
        self.base_ring().get_ring().has_canonical_iso(from.base_ring().get_ring())
    }

    fn map_out(&self, from: &SparsePolyRingBase<R1>, el: Self::Element, iso: &Self::Isomorphism) -> SparseVectorMut<Rc<R1>> {
        let mut result = SparseVectorMut::new(el.len(), from.base_ring.clone());
        for (j, c) in el.nontrivial_entries() {
            *result.at_mut(j) = self.base_ring().get_ring().map_out(from.base_ring().get_ring(), self.base_ring().clone_el(c), iso);
        }
        return result;
    }
}

pub struct TermIterator<'a, R>
    where R: RingStore
{
    iter: sparse::SparseVectorMutIter<'a, Rc<R>>
}

impl<'a, R> Iterator for TermIterator<'a, R>
    where R: RingStore
{
    type Item = (&'a El<R>, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((i, c)) = self.iter.next() {
            Some((c, i))
        } else {
            None
        }
    }
}

impl<R> PolyRing for SparsePolyRingBase<R> 
    where R: RingStore
{
    type TermsIterator<'a> = TermIterator<'a, R>
        where Self: 'a;

    fn indeterminate(&self) -> Self::Element {
        let mut result = self.zero();
        result.set_len(2);
        *result.at_mut(1) = self.base_ring.one();
        return result;
    }

    fn terms<'a>(&'a self, f: &'a Self::Element) -> TermIterator<'a, R> {
        TermIterator {
            iter: f.nontrivial_entries()
        }
    }

    fn add_assign_from_terms<I>(&self, lhs: &mut Self::Element, rhs: I)
        where I: Iterator<Item = (El<Self::BaseRing>, usize)>
    {
        for (c, i) in rhs {
            lhs.set_len(max(lhs.len(), i + 1));
            self.base_ring().add_assign(lhs.at_mut(i), c);
        }
        // if a previously set entry is then set to zero or adds up to zero, this might be not truncated
        self.degree_truncate(lhs);
    }

    fn coefficient_at<'a>(&'a self, f: &'a Self::Element, i: usize) -> &'a El<Self::BaseRing> {
        if i < f.len() {
            return f.at(i);
        } else {
            return &self.zero;
        }
    }

    fn degree(&self, f: &Self::Element) -> Option<usize> {
        f.len().checked_sub(1)
    }

    fn div_rem_monic(&self, mut lhs: Self::Element, rhs: &Self::Element) -> (Self::Element, Self::Element) {
        assert!(self.base_ring().is_one(self.coefficient_at(rhs, self.degree(rhs).unwrap())));
        let quo = self.poly_div(&mut lhs, rhs, |x| Some(x)).unwrap();
        return (quo, lhs);
    }
}

impl<R,> DivisibilityRing for SparsePolyRingBase<R> 
    where R: DivisibilityRingStore, R::Type: DivisibilityRing
{
    fn checked_left_div(&self, lhs: &Self::Element, rhs: &Self::Element) -> Option<Self::Element> {
        if let Some(d) = self.degree(rhs) {
            let lc = rhs.at(d);
            let mut lhs_copy = lhs.clone();
            let quo = self.poly_div(&mut lhs_copy, rhs, |x| self.base_ring().checked_left_div(&x, lc))?;
            if self.is_zero(&lhs_copy) {
                Some(quo)
            } else {
                None
            }
        } else if self.is_zero(lhs) {
            Some(self.zero())
        } else {
            None
        }
    }
}

impl<R> PrincipalIdealRing for SparsePolyRingBase<R>
    where R: RingStore, R::Type: Field
{
    fn ideal_gen(&self, lhs: &Self::Element, rhs: &Self::Element) -> (Self::Element, Self::Element, Self::Element) {
        algorithms::eea::eea(self.clone_el(lhs), self.clone_el(rhs), RingRef::new(self))
    }
}

impl<R> EuclideanRing for SparsePolyRingBase<R> 
    where R: RingStore, R::Type: Field
{
    fn euclidean_div_rem(&self, mut lhs: Self::Element, rhs: &Self::Element) -> (Self::Element, Self::Element) {
        let lc_inv = self.base_ring.invert(rhs.at(self.degree(rhs).unwrap())).unwrap();
        let quo = self.poly_div(&mut lhs, rhs, |x| Some(self.base_ring().mul_ref_snd(x, &lc_inv))).unwrap();
        return (quo, lhs);
    }

    fn euclidean_deg(&self, val: &Self::Element) -> Option<usize> {
        return Some(self.degree(val).map(|x| x + 1).unwrap_or(0));
    }
}

#[cfg(test)]
use crate::rings::zn::*;
#[cfg(test)]
use crate::rings::zn::zn_static::{Zn, Fp};
#[cfg(test)]
use crate::rings::finite::FiniteRingStore;
#[cfg(test)]
use super::dense_poly::DensePolyRing;
#[cfg(test)]
use crate::primitive_int::StaticRing;

#[cfg(test)]
fn edge_case_elements<P: PolyRingStore>(poly_ring: P) -> impl Iterator<Item = El<P>>
    where P::Type: PolyRing
{
    let base_ring = poly_ring.base_ring();
    vec![ 
        poly_ring.from_terms([].into_iter()),
        poly_ring.from_terms([(base_ring.int_hom().map(1), 0)].into_iter()),
        poly_ring.from_terms([(base_ring.int_hom().map(1), 1)].into_iter()),
        poly_ring.from_terms([(base_ring.int_hom().map(1), 0), (base_ring.int_hom().map(1), 1)].into_iter()),
        poly_ring.from_terms([(base_ring.int_hom().map(-1), 0)].into_iter()),
        poly_ring.from_terms([(base_ring.int_hom().map(-1), 1)].into_iter()),
        poly_ring.from_terms([(base_ring.int_hom().map(-1), 0), (base_ring.int_hom().map(1), 1)].into_iter()),
        poly_ring.from_terms([(base_ring.int_hom().map(1), 0), (base_ring.int_hom().map(-1), 1)].into_iter()),
        poly_ring.from_terms([(base_ring.int_hom().map(-1), 0), (base_ring.int_hom().map(1), 2)].into_iter()),
        poly_ring.from_terms([(base_ring.int_hom().map(1), 0), (base_ring.int_hom().map(-1), 2)].into_iter()),
        poly_ring.from_terms([(base_ring.int_hom().map(1), 0), (base_ring.int_hom().map(-1), 2), (base_ring.int_hom().map(0), 2)].into_iter())
    ].into_iter()
}

#[test]
fn test_ring_axioms() {
    let poly_ring = SparsePolyRing::new(Zn::<7>::RING, "X");
    crate::ring::generic_tests::test_ring_axioms(&poly_ring, edge_case_elements(&poly_ring));
}

#[test]
fn test_poly_ring_axioms() {
    let poly_ring = SparsePolyRing::new(Zn::<7>::RING, "X");
    super::generic_tests::test_poly_ring_axioms(poly_ring, Zn::<7>::RING.elements());
}

#[test]
fn test_canonical_iso_axioms_different_base_ring() {
    let poly_ring1 = SparsePolyRing::new(zn_barett::Zn::new(StaticRing::<i128>::RING, 7), "X");
    let poly_ring2 = SparsePolyRing::new(zn_42::Zn::new(7), "X");
    crate::ring::generic_tests::test_hom_axioms(&poly_ring1, &poly_ring2, edge_case_elements(&poly_ring1));
    crate::ring::generic_tests::test_iso_axioms(&poly_ring1, &poly_ring2, edge_case_elements(&poly_ring1));
}

#[test]
fn test_canonical_iso_dense_poly_ring() {
    let poly_ring1 = SparsePolyRing::new(zn_42::Zn::new(7), "X");
    let poly_ring2 = DensePolyRing::new(zn_42::Zn::new(7), "X");
    crate::ring::generic_tests::test_hom_axioms(&poly_ring2, &poly_ring1, edge_case_elements(&poly_ring2));
    crate::ring::generic_tests::test_iso_axioms(&poly_ring2, &poly_ring1, edge_case_elements(&poly_ring2));
}

#[test]
fn test_divisibility_ring_axioms() {
    let poly_ring = SparsePolyRing::new(Zn::<7>::RING, "X");
    crate::divisibility::generic_tests::test_divisibility_axioms(&poly_ring, edge_case_elements(&poly_ring));
}

#[test]
fn test_euclidean_ring_axioms() {
    let poly_ring = SparsePolyRing::new(Fp::<7>::RING, "X");
    crate::pid::generic_tests::test_euclidean_ring_axioms(&poly_ring, edge_case_elements(&poly_ring));
}