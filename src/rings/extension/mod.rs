use crate::ring::*;
use crate::vector::vec_fn::*;
use crate::homomorphism::*;
use super::poly::{PolyRingStore, PolyRing};

pub mod extension_impl;

///
/// A ring `R` that is an extension of a base ring `S`, generated by a single element
/// that is algebraic resp. integral over `S`. Furthermore, `R` must be a free `S`-module,
/// with a basis given by the powers of [`FreeAlgebra::canonical_gen()`].
/// 
/// # Nontrivial Automorphisms
/// 
/// Rings of this form very often have nontrivial automorphisms. In order to simplify situations
/// where morphisms or other objects are only unique up to isomorphism, morphisms between rings
/// of this type must also preserve the canonical generator. 
/// 
pub trait FreeAlgebra: RingExtension {

    type VectorRepresentation<'a>: VectorFn<El<Self::BaseRing>>
        where Self: 'a;

    fn canonical_gen(&self) -> Self::Element;
    fn rank(&self) -> usize;
    fn wrt_canonical_basis<'a>(&'a self, el: &'a Self::Element) -> Self::VectorRepresentation<'a>;

    fn from_canonical_basis<V>(&self, vec: V) -> Self::Element
        where V: ExactSizeIterator + DoubleEndedIterator + Iterator<Item = El<Self::BaseRing>>
    {
        assert_eq!(vec.len(), self.rank());
        let x = self.canonical_gen();
        let mut result = self.zero();
        for c in vec.rev() {
            self.mul_assign_ref(&mut result, &x);
            self.add_assign(&mut result, self.from(c));
        }
        return result;
    }
}

pub trait FreeAlgebraStore: RingStore
    where Self::Type: FreeAlgebra
{
    delegate!{ fn canonical_gen(&self) -> El<Self> }
    delegate!{ fn rank(&self) -> usize }

    fn wrt_canonical_basis<'a>(&'a self, el: &'a El<Self>) -> <Self::Type as FreeAlgebra>::VectorRepresentation<'a> {
        self.get_ring().wrt_canonical_basis(el)
    }

    fn from_canonical_basis<V>(&self, vec: V) -> El<Self>
        where V: ExactSizeIterator + DoubleEndedIterator + Iterator<Item = El<<Self::Type as RingExtension>::BaseRing>>
    {
        self.get_ring().from_canonical_basis(vec)
    }
}

impl<R: RingStore> FreeAlgebraStore for R
    where R::Type: FreeAlgebra
{}

pub fn poly_repr<P: PolyRingStore, R: FreeAlgebraStore>(from: R, to: P, el: &El<R>) -> El<P>
    where P::Type: PolyRing, 
        R::Type: FreeAlgebra,
        <<P::Type as RingExtension>::BaseRing as RingStore>::Type: CanHomFrom<<<R::Type as RingExtension>::BaseRing as RingStore>::Type>
{
    let hom = to.base_ring().can_hom(from.base_ring()).unwrap();
    let coeff_vec = from.wrt_canonical_basis(el);
    to.from_terms(
        (0..from.rank()).map(|i| coeff_vec.at(i)).enumerate()
            .filter(|(_, x)| !from.base_ring().is_zero(x))
            .map(|(j, x)| (hom.map(x), j))
    )

}

#[cfg(any(test, feature = "generic_tests"))]
pub fn generic_test_free_algebra_axioms<R: FreeAlgebraStore>(ring: R)
    where R::Type: FreeAlgebra
{
    let x = ring.canonical_gen();
    let n = ring.rank();
    
    let xn_original = ring.pow(ring.clone_el(&x), n);
    let xn_vec = ring.wrt_canonical_basis(&xn_original);
    let xn = ring.sum(Iterator::map(0..n, |i| ring.mul(ring.inclusion().map(xn_vec.at(i)), ring.pow(ring.clone_el(&x), i))));
    assert_el_eq!(&ring, &xn_original, &xn);

    let x_n_1_vec_expected = (0..n).into_fn().map(|i| if i > 0 {
        ring.base_ring().add(ring.base_ring().mul(xn_vec.at(n - 1), xn_vec.at(i)), xn_vec.at(i - 1))
    } else {
        ring.base_ring().mul(xn_vec.at(n - 1), xn_vec.at(0))
    });
    let x_n_1 = ring.pow(ring.clone_el(&x), n + 1);
    let x_n_1_vec_actual = ring.wrt_canonical_basis(&x_n_1);
    for i in 0..n {
        assert_el_eq!(ring.base_ring(), &x_n_1_vec_expected.at(i), &x_n_1_vec_actual.at(i));
    }

    // test basis wrt_root_of_unity_basis linearity and compatibility from_root_of_unity_basis/wrt_root_of_unity_basis
    for i in (0..ring.rank()).step_by(5) {
        for j in (1..ring.rank()).step_by(7) {
            if i == j {
                continue;
            }
            let element = ring.from_canonical_basis(Iterator::map(0..n, |k| if k == i { ring.base_ring().one() } else if k == j { ring.base_ring().int_hom().map(2) } else { ring.base_ring().zero() }));
            let expected = ring.add(ring.pow(ring.clone_el(&x), i), ring.int_hom().mul_map(ring.pow(ring.clone_el(&x), j), 2));
            assert_el_eq!(&ring, &expected, &element);
            let element_vec = ring.wrt_canonical_basis(&expected);
            for k in 0..ring.rank() {
                if k == i {
                    assert_el_eq!(ring.base_ring(), &ring.base_ring().one(), &element_vec.at(k));
                } else if k == j {
                    assert_el_eq!(ring.base_ring(), &ring.base_ring().int_hom().map(2), &element_vec.at(k));
                } else {
                    assert_el_eq!(ring.base_ring(), &ring.base_ring().zero(), &element_vec.at(k));
                }
            }
        }
    }
}
