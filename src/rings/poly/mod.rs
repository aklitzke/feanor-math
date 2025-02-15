use crate::ring::*;
use crate::homomorphism::*;

pub mod dense_poly;
pub mod sparse_poly;

///
/// Trait for all rings that represent the polynomial ring `R[X]` with
/// any base ring R.
/// 
/// Currently, the two implementations of this type of ring are [`dense_poly::DensePolyRing`]
/// and [`sparse_poly::SparsePolyRing`].
/// 
pub trait PolyRing: RingExtension {

    type TermsIterator<'a>: Iterator<Item = (&'a El<Self::BaseRing>, usize)>
        where Self: 'a;

    fn indeterminate(&self) -> Self::Element;

    fn terms<'a>(&'a self, f: &'a Self::Element) -> Self::TermsIterator<'a>;
    
    fn add_assign_from_terms<I>(&self, lhs: &mut Self::Element, rhs: I)
        where I: Iterator<Item = (El<Self::BaseRing>, usize)>
    {
        let self_ring = RingRef::new(self);
        self.add_assign(lhs, self_ring.sum(
            rhs.map(|(c, i)| self.mul(self.from(c), self_ring.pow(self.indeterminate(), i)))
        ));
    }

    fn coefficient_at<'a>(&'a self, f: &'a Self::Element, i: usize) -> &'a El<Self::BaseRing>;

    fn degree(&self, f: &Self::Element) -> Option<usize>;

    fn div_rem_monic(&self, lhs: Self::Element, rhs: &Self::Element) -> (Self::Element, Self::Element);
    
    fn map_terms<P, H>(&self, from: &P, el: &P::Element, hom: &H) -> Self::Element
        where P: ?Sized + PolyRing,
            H: Homomorphism<<P::BaseRing as RingStore>::Type, <Self::BaseRing as RingStore>::Type>
    {
        assert!(self.base_ring().get_ring() == hom.codomain().get_ring());
        assert!(from.base_ring().get_ring() == hom.domain().get_ring());
        RingRef::new(self).from_terms(from.terms(el).map(|(c, i)| (hom.map_ref(c), i)))
    }

    fn evaluate<R, H>(&self, f: &Self::Element, value: &R::Element, hom: &H) -> R::Element
        where R: ?Sized + RingBase,
            H: Homomorphism<<Self::BaseRing as RingStore>::Type, R>
    {
        hom.codomain().sum(self.terms(f).map(|(c, i)| {
            let result = hom.codomain().pow(hom.codomain().clone_el(value), i);
            hom.mul_ref_snd_map(result, c)
        }))
    }
}

pub trait PolyRingStore: RingStore
    where Self::Type: PolyRing
{
    delegate!{ fn indeterminate(&self) -> El<Self> }
    delegate!{ fn degree(&self, f: &El<Self>) -> Option<usize> }
    delegate!{ fn div_rem_monic(&self, lhs: El<Self>, rhs: &El<Self>) -> (El<Self>, El<Self>) }

    fn coefficient_at<'a>(&'a self, f: &'a El<Self>, i: usize) -> &'a El<<Self::Type as RingExtension>::BaseRing> {
        self.get_ring().coefficient_at(f, i)
    }

    fn terms<'a>(&'a self, f: &'a El<Self>) -> <Self::Type as PolyRing>::TermsIterator<'a> {
        self.get_ring().terms(f)
    }

    fn from_terms<I>(&self, iter: I) -> El<Self>
        where I: Iterator<Item = (El<<Self::Type as RingExtension>::BaseRing>, usize)>,
    {
        let mut result = self.zero();
        self.get_ring().add_assign_from_terms(&mut result, iter);
        return result;
    }

    fn lc<'a>(&'a self, f: &'a El<Self>) -> Option<&'a El<<Self::Type as RingExtension>::BaseRing>> {
        Some(self.coefficient_at(f, self.degree(f)?))
    }

    fn evaluate<R, H>(&self, f: &El<Self>, value: &R::Element, hom: &H) -> R::Element
        where R: ?Sized + RingBase,
            H: Homomorphism<<<Self::Type as RingExtension>::BaseRing as RingStore>::Type, R>
    {
        self.get_ring().evaluate(f, value, hom)
    }
}

impl<R: RingStore> PolyRingStore for R
    where R::Type: PolyRing
{}

pub mod generic_impls {
    use crate::ring::*;
    use super::PolyRing;
    use crate::homomorphism::*;

    #[allow(type_alias_bounds)]
    pub type GenericCanHomFrom<P1: PolyRing, P2: PolyRing> = <<P2::BaseRing as RingStore>::Type as CanHomFrom<<P1::BaseRing as RingStore>::Type>>::Homomorphism;

    pub fn generic_has_canonical_hom<P1: PolyRing, P2: PolyRing>(from: &P1, to: &P2) -> Option<GenericCanHomFrom<P1, P2>> 
        where <P2::BaseRing as RingStore>::Type: CanHomFrom<<P1::BaseRing as RingStore>::Type>
    {
        to.base_ring().get_ring().has_canonical_hom(from.base_ring().get_ring())
    }

    pub fn generic_map_in<P1: PolyRing, P2: PolyRing>(from: &P1, to: &P2, el: P1::Element, hom: &GenericCanHomFrom<P1, P2>) -> P2::Element
        where <P2::BaseRing as RingStore>::Type: CanHomFrom<<P1::BaseRing as RingStore>::Type>
    {
        let mut result = to.zero();
        to.add_assign_from_terms(&mut result, from.terms(&el).map(|(c, i)| (to.base_ring().get_ring().map_in(from.base_ring().get_ring(), from.base_ring().clone_el(c), hom), i)));
        return result;
    }

    #[allow(type_alias_bounds)]
    pub type GenericCanonicalIso<P1: PolyRing, P2: PolyRing> = <<P2::BaseRing as RingStore>::Type as CanonicalIso<<P1::BaseRing as RingStore>::Type>>::Isomorphism;

    pub fn generic_map_out<P1: PolyRing, P2: PolyRing>(from: &P1, to: &P2, el: P2::Element, iso: &GenericCanonicalIso<P1, P2>) -> P1::Element
        where <P2::BaseRing as RingStore>::Type: CanonicalIso<<P1::BaseRing as RingStore>::Type>
    {
        let mut result = from.zero();
        from.add_assign_from_terms(&mut result, to.terms(&el).map(|(c, i)| (to.base_ring().get_ring().map_out(from.base_ring().get_ring(), to.base_ring().clone_el(c), iso), i)));
        return result;
    }

    pub fn dbg_poly<P: PolyRing>(ring: &P, el: &P::Element, out: &mut std::fmt::Formatter, unknown_name: &str) -> std::fmt::Result {
        let mut terms = ring.terms(el);
        let print_unknown = |i: usize, out: &mut std::fmt::Formatter| {
            if i == 0 {
                // print nothing
                Ok(())
            } else if i == 1 {
                write!(out, "{}", unknown_name)
            } else {
                write!(out, "{}^{}", unknown_name, i)
            }
        };
        if let Some((c, i)) = terms.next() {
            ring.base_ring().get_ring().dbg(c, out)?;
            print_unknown(i, out)?;
        } else {
            write!(out, "0")?;
        }
        while let Some((c, i)) = terms.next() {
            write!(out, " + ")?;
            ring.base_ring().get_ring().dbg(c, out)?;
            print_unknown(i, out)?;
        }
        return Ok(());
    }
}

#[cfg(any(test, feature = "generic_tests"))]
pub mod generic_tests {
    use super::*;

    pub fn test_poly_ring_axioms<R: PolyRingStore, I: Iterator<Item = El<<R::Type as RingExtension>::BaseRing>>>(ring: R, interesting_base_ring_elements: I)
        where R::Type: PolyRing
    {    
        let x = ring.indeterminate();
        let elements = interesting_base_ring_elements.collect::<Vec<_>>();
        
        // test linear independence of X
        for a in &elements {
            for b in &elements {
                for c in &elements {
                    for d in &elements {
                        let a_bx = ring.add(ring.inclusion().map_ref(a), ring.mul_ref_snd(ring.inclusion().map_ref(b), &x));
                        let c_dx = ring.add(ring.inclusion().map_ref(c), ring.mul_ref_snd(ring.inclusion().map_ref(d), &x));
                        assert!(ring.eq_el(&a_bx, &c_dx) == (ring.base_ring().eq_el(a, c) && ring.base_ring().eq_el(b, d)));
                    }
                }
            }
        }
        
        // elementwise addition follows trivially from the ring axioms

        // technically, convoluted multiplication follows from the ring axioms too, but test it anyway
        for a in &elements {
            for b in &elements {
                for c in &elements {
                    for d in &elements {
                        let a_bx = ring.add(ring.inclusion().map_ref(a), ring.mul_ref_snd(ring.inclusion().map_ref(b), &x));
                        let c_dx = ring.add(ring.inclusion().map_ref(c), ring.mul_ref_snd(ring.inclusion().map_ref(d), &x));
                        let result = <_ as RingStore>::sum(&ring, [
                            ring.mul(ring.inclusion().map_ref(a), ring.inclusion().map_ref(c)),
                            ring.mul(ring.inclusion().map_ref(a), ring.mul_ref_snd(ring.inclusion().map_ref(d), &x)),
                            ring.mul(ring.inclusion().map_ref(b), ring.mul_ref_snd(ring.inclusion().map_ref(c), &x)),
                            ring.mul(ring.inclusion().map_ref(b), ring.mul(ring.inclusion().map_ref(d), ring.pow(ring.clone_el(&x), 2)))
                        ].into_iter());
                        assert_el_eq!(&ring, &result, &ring.mul(a_bx, c_dx));
                    }
                }
            }
        }

        // test terms(), from_terms()
        for a in &elements {
            for b in &elements {
                for c in &elements {
                    let f = <_ as RingStore>::sum(&ring, [
                        ring.inclusion().map_ref(a),
                        ring.mul_ref_snd(ring.inclusion().map_ref(b), &x),
                        ring.mul(ring.inclusion().map_ref(c), ring.pow(ring.clone_el(&x), 3))
                    ].into_iter());
                    let actual = ring.from_terms([(ring.base_ring().clone_el(a), 0), (ring.base_ring().clone_el(c), 3), (ring.base_ring().clone_el(b), 1)].into_iter());
                    assert_el_eq!(&ring, &f, &actual);
                    assert_el_eq!(&ring, &f, &ring.from_terms(ring.terms(&f).map(|(c, i)| (ring.base_ring().clone_el(c), i))));
                }
            }
        }

        // test div_rem_monic()
        for a in &elements {
            for b in &elements {
                for c in &elements {
                    let f = ring.from_terms([(ring.base_ring().clone_el(a), 0), (ring.base_ring().clone_el(b), 3)].into_iter());
                    let g = ring.from_terms([(ring.base_ring().negate(ring.base_ring().clone_el(c)), 0), (ring.base_ring().one(), 1)].into_iter());

                    let (quo, rem) = ring.div_rem_monic(ring.clone_el(&f), &g);
                    assert_el_eq!(
                        &ring,
                        &ring.from_terms([(ring.base_ring().add_ref_fst(a, ring.base_ring().mul_ref_fst(b, ring.base_ring().pow(ring.base_ring().clone_el(c), 3))), 0)].into_iter()),
                        &rem
                    );
                    assert_el_eq!(
                        &ring,
                        &f,
                        &ring.add(rem, ring.mul(quo, g))
                    );
                }
            }
        }

        // test evaluate()
        let hom = ring.base_ring().int_hom();
        let base_ring = hom.codomain();
        let f = ring.from_terms([(hom.map(1), 0), (hom.map(3), 1), (hom.map(7), 3)].into_iter());
        for a in &elements {
            assert_el_eq!(
                &base_ring,
                &base_ring.add(base_ring.one(), base_ring.add(base_ring.mul_ref_snd(hom.map(3), a), base_ring.mul(hom.map(7), base_ring.pow(base_ring.clone_el(a), 3)))),
                &ring.evaluate(&f, a, &base_ring.identity())
            )
        }
    }
}