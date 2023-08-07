use crate::mempool::MemoryProvider;
use crate::rings::poly::PolyRing;
use crate::rings::poly::PolyRingStore;
use crate::rings::poly::dense_poly::DensePolyRing;
use crate::vector::VectorViewSparse;
use crate::vector::vec_fn::RingElVectorViewFn;
use crate::ring::*;
use crate::algorithms;
use crate::vector::VectorView;
use crate::mempool::AllocatingMemoryProvider;

use super::*;

pub struct FreeAlgebraImplBase<R, V, M>
    where R: RingStore, V: VectorView<El<R>>, M: MemoryProvider<El<R>>
{
    base_ring: R,
    x_pow_rank: V,
    memory_provider: M
}

pub struct FreeAlgebraEl<R, M>
    where R: RingStore, M: MemoryProvider<El<R>>
{
    values: M::Object
}

pub type FreeAlgebraImpl<R, V, M = AllocatingMemoryProvider> = RingValue<FreeAlgebraImplBase<R, V, M>>;

impl<R, V, M> FreeAlgebraImpl<R, V, M>
    where R: RingStore, V: VectorView<El<R>>, M: MemoryProvider<El<R>>
{
    pub fn new(base_ring: R, x_pow_rank: V, memory_provider: M) -> FreeAlgebraImpl<R, V, M> {
        RingValue::from(FreeAlgebraImplBase {
            base_ring, x_pow_rank, memory_provider
        })
    }
}

impl<R, V, M> FreeAlgebraImplBase<R, V, M>
    where R: RingStore, V: VectorView<El<R>>, M: MemoryProvider<El<R>>
{
    pub fn poly_repr<P>(&self, poly_ring: P, el: &<Self as RingBase>::Element) -> El<P>
        where P: PolyRingStore,
            P::Type: PolyRing,
            <<P::Type as RingExtension>::BaseRing as RingStore>::Type: CanonicalHom<R::Type>
    {
        let hom = poly_ring.base_ring().can_hom(self.base_ring()).unwrap();
        poly_ring.from_terms(Iterator::map(0..self.rank(), |i| (hom.map(self.base_ring.clone_el(el.values.at(i))), i)))
    }
}

impl<R, V, M> PartialEq for FreeAlgebraImplBase<R, V, M>
    where R: RingStore, V: VectorView<El<R>>, M: MemoryProvider<El<R>>
{
    fn eq(&self, other: &Self) -> bool {
        self.base_ring.get_ring() == other.base_ring.get_ring() && self.rank() == other.rank() && (0..self.rank()).all(|i| self.base_ring.eq_el(self.x_pow_rank.at(i), other.x_pow_rank.at(i)))
    }
}

impl<R, V, M> RingBase for FreeAlgebraImplBase<R, V, M>
    where R: RingStore, V: VectorView<El<R>>, M: MemoryProvider<El<R>>
{
    type Element = FreeAlgebraEl<R, M>;

    fn clone_el(&self, val: &Self::Element) -> Self::Element {
        FreeAlgebraEl { values: self.memory_provider.get_new_init(self.rank(), |i| self.base_ring.clone_el(val.values.at(i))) }
    }

    fn add_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) {
        for i in 0..self.rank() {
            self.base_ring.add_assign_ref(&mut lhs.values[i], &rhs.values[i]);
        }
    }

    fn add_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) {
        self.add_assign_ref(lhs, &rhs);
    }

    fn sub_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) {
        for i in 0..self.rank() {
            self.base_ring.sub_assign_ref(&mut lhs.values[i], &rhs.values[i]);
        }
    }

    fn negate_inplace(&self, lhs: &mut Self::Element) {
        for i in 0..self.rank() {
            self.base_ring.negate_inplace(&mut lhs.values[i]);
        }
    }

    fn mul_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) {
        self.mul_assign_ref(lhs, &rhs);
    }

    default fn mul_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) {
        let mut tmp = self.memory_provider.get_new_init(self.rank() * 2, |_| self.base_ring.zero());
        algorithms::conv_mul::add_assign_convoluted_mul(&mut tmp[..], &lhs.values[..], &rhs.values[..], self.base_ring(), &self.memory_provider);
        for i in self.rank()..tmp.len() {
            for j in 0..self.rank() {
                let add = self.base_ring.mul_ref(self.x_pow_rank.at(j), &tmp[i]);
                self.base_ring.add_assign(&mut tmp[i - self.rank() + j], add);
            }
        }
        for i in 0..self.rank() {
            lhs.values[i] = std::mem::replace(&mut tmp[i], self.base_ring.zero());
        }
    }

    fn from_int(&self, value: i32) -> Self::Element {
        self.from(self.base_ring.from_int(value))
    }

    fn eq_el(&self, lhs: &Self::Element, rhs: &Self::Element) -> bool {
        (0..self.rank()).all(|i| self.base_ring.eq_el(lhs.values.at(i), rhs.values.at(i)))
    }
    
    fn is_commutative(&self) -> bool {
        self.base_ring.is_commutative()
    }

    fn is_noetherian(&self) -> bool {
        self.base_ring.is_noetherian()
    }
    
    fn dbg<'a>(&self, value: &Self::Element, out: &mut std::fmt::Formatter<'a>) -> std::fmt::Result {
        let poly_ring = DensePolyRing::new(self.base_ring(), "θ");
        poly_ring.get_ring().dbg(&self.poly_repr(&poly_ring, value), out)
    }

    fn mul_assign_int(&self, lhs: &mut Self::Element, rhs: i32) {
        self.mul_assign(lhs, self.from_int(rhs));
    }
}

impl<R, V, M> RingBase for FreeAlgebraImplBase<R, V, M>
    where R: RingStore, V: VectorViewSparse<El<R>>, M: MemoryProvider<El<R>>
{
    fn mul_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) {
        let mut tmp = self.memory_provider.get_new_init(self.rank() * 2, |_| self.base_ring.zero());
        algorithms::conv_mul::add_assign_convoluted_mul(&mut tmp[..], &lhs.values[..], &rhs.values[..], self.base_ring(), &self.memory_provider);
        for i in self.rank()..tmp.len() {
            for (j, c) in self.x_pow_rank.nontrivial_entries() {
                let add = self.base_ring.mul_ref(c, &tmp[i]);
                self.base_ring.add_assign(&mut tmp[i - self.rank() + j], add);
            }
        }
        for i in 0..self.rank() {
            lhs.values[i] = std::mem::replace(&mut tmp[i], self.base_ring.zero());
        }
    }
}

impl<R, V, M> RingExtension for FreeAlgebraImplBase<R, V, M>
    where R: RingStore, V: VectorView<El<R>>, M: MemoryProvider<El<R>>
{
    type BaseRing = R;

    fn base_ring<'a>(&'a self) -> &'a Self::BaseRing {
        &self.base_ring
    }

    fn from(&self, x: El<Self::BaseRing>) -> Self::Element {
        let mut data = Some(x);
        FreeAlgebraEl { values: self.memory_provider.get_new_init(self.rank(), |i| if i == 0 { std::mem::replace(&mut data, None).unwrap() } else { self.base_ring.zero() }) }
    }

    fn mul_assign_base(&self, lhs: &mut Self::Element, rhs: &El<Self::BaseRing>) {
        for i in 0..self.rank() {
            self.base_ring.mul_assign_ref(&mut lhs.values[i], rhs);
        }
    }
}

impl<R, V, M> FreeAlgebra for FreeAlgebraImplBase<R, V, M>
    where R: RingStore, V: VectorView<El<R>>, M: MemoryProvider<El<R>>
{
    type VectorRepresentation<'a> = RingElVectorViewFn<&'a R, &'a [El<R>], El<R>>
        where Self: 'a;

    fn canonical_gen(&self) -> Self::Element {
        FreeAlgebraEl { values: self.memory_provider.get_new_init(self.rank(), |i| if i == 1 { self.base_ring.one() } else { self.base_ring.zero() }) }
    }

    fn wrt_canonical_basis<'a>(&'a self, el: &'a Self::Element) -> Self::VectorRepresentation<'a> {
        (&el.values[..]).as_el_fn(self.base_ring())
    }

    fn rank(&self) -> usize {
        self.x_pow_rank.len()
    }
}

impl<R1, V1, M1, R2, V2, M2> CanonicalHom<FreeAlgebraImplBase<R1, V1, M1>> for FreeAlgebraImplBase<R2, V2, M2>
    where R1: RingStore, V1: VectorView<El<R1>>, M1: MemoryProvider<El<R1>>,
        R2: RingStore, V2: VectorView<El<R2>>, M2: MemoryProvider<El<R2>>,
        R2::Type: CanonicalHom<R1::Type>
{
    type Homomorphism = <R2::Type as CanonicalHom<R1::Type>>::Homomorphism;

    fn has_canonical_hom(&self, from: &FreeAlgebraImplBase<R1, V1, M1>) -> Option<Self::Homomorphism> {
        if self.rank() == from.rank() {
            let hom = self.base_ring.get_ring().has_canonical_hom(from.base_ring.get_ring())?;
            if (0..self.rank()).all(|i| self.base_ring.eq_el(self.x_pow_rank.at(i), &self.base_ring.get_ring().map_in_ref(from.base_ring.get_ring(), from.x_pow_rank.at(i), &hom))) {
                Some(hom)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn map_in(&self, from: &FreeAlgebraImplBase<R1, V1, M1>, el: <FreeAlgebraImplBase<R1, V1, M1> as RingBase>::Element, hom: &Self::Homomorphism) -> Self::Element {
        FreeAlgebraEl { values: self.memory_provider.get_new_init(self.rank(), |i| self.base_ring.get_ring().map_in_ref(from.base_ring.get_ring(), &el.values[i], hom)) }
    }
}

impl<R1, V1, M1, R2, V2, M2> CanonicalIso<FreeAlgebraImplBase<R1, V1, M1>> for FreeAlgebraImplBase<R2, V2, M2>
    where R1: RingStore, V1: VectorView<El<R1>>, M1: MemoryProvider<El<R1>>,
        R2: RingStore, V2: VectorView<El<R2>>, M2: MemoryProvider<El<R2>>,
        R2::Type: CanonicalIso<R1::Type>
{
    type Isomorphism = <R2::Type as CanonicalIso<R1::Type>>::Isomorphism;

    fn has_canonical_iso(&self, from: &FreeAlgebraImplBase<R1, V1, M1>) -> Option<Self::Isomorphism> {
        if self.rank() == from.rank() {
            let iso = self.base_ring.get_ring().has_canonical_iso(from.base_ring.get_ring())?;
            if (0..self.rank()).all(|i| from.base_ring.eq_el(&self.base_ring.get_ring().map_out(from.base_ring.get_ring(), self.base_ring.clone_el(self.x_pow_rank.at(i)), &iso), from.x_pow_rank.at(i))) {
                Some(iso)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn map_out(&self, from: &FreeAlgebraImplBase<R1, V1, M1>, el: <FreeAlgebraImplBase<R2, V2, M2> as RingBase>::Element, iso: &Self::Isomorphism) -> <FreeAlgebraImplBase<R1, V1, M1> as RingBase>::Element {
        FreeAlgebraEl { values: from.memory_provider.get_new_init(self.rank(), |i| self.base_ring.get_ring().map_out(from.base_ring.get_ring(), self.base_ring.clone_el(&el.values[i]), iso)) }
    }
}

#[cfg(test)]
use crate::primitive_int::StaticRing;

#[cfg(test)]
fn test_ring_and_elements() -> (FreeAlgebraImpl<StaticRing::<i64>, [i64; 2], AllocatingMemoryProvider>, Vec<FreeAlgebraEl<StaticRing<i64>, AllocatingMemoryProvider>>) {
    let ZZ = StaticRing::<i64>::RING;
    let R = FreeAlgebraImpl::new(ZZ, [1, 1], AllocatingMemoryProvider);
    let mut elements = Vec::new();
    for a in -3..=3 {
        for b in -3..=3 {
            elements.push(R.from_canonical_basis([a, b].into_iter()));
        }
    }
    return (R, elements);
}

#[test]
fn test_ring_axioms() {
    let (ring, els) = test_ring_and_elements();
    generic_test_ring_axioms(ring, els.into_iter());
}

#[test]
fn test_free_algebra_axioms() {
    let (ring, _) = test_ring_and_elements();
    generic_test_free_algebra_axioms(ring);
}