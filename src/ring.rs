use std::ops::Deref;

use crate::{algorithms, primitive_int::{StaticRing}, integer::IntegerRingStore};

///
/// Basic trait for objects that have a ring structure.
/// 
/// Implementors of this trait should provide the basic ring operations,
/// and additionally operators for displaying and equality testing. If
/// a performance advantage can be achieved by accepting some arguments by
/// reference instead of by value, the default-implemented functions for
/// ring operations on references should be overwritten.
/// 
/// # Relationship with [`RingStore`]
/// 
/// Note that usually, this trait will not be used directly, but always
/// through a [`RingStore`]. In more detail, while this trait
/// defines the functionality, [`RingStore`] allows abstracting
/// the storage - everything that allows access to a ring then is a 
/// [`RingStore`], as for example, references or shared pointers
/// to rings. If you want to use rings directly by value, some technical
/// details make it necessary to use the no-op container [`RingStore`].
/// For more detail, see the documentation of [`RingStore`].
///  
/// # Example
/// 
/// An example implementation of a new, very useless ring type that represents
/// 32-bit integers stored on the heap.
/// ```
/// # use feanor_math::ring::*;
/// struct MyRingBase;
/// 
/// impl RingBase for MyRingBase {
///     
///     type Element = Box<i32>;
///
///     fn add_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) { **lhs += *rhs; }
/// 
///     fn negate_inplace(&self, lhs: &mut Self::Element) { **lhs = -**lhs; }
/// 
///     fn mul_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) { **lhs *= *rhs; }
/// 
///     fn from_z(&self, value: i32) -> Self::Element { Box::new(value) }
/// 
///     fn eq(&self, lhs: &Self::Element, rhs: &Self::Element) -> bool { **lhs == **rhs }
/// 
///     fn is_commutative(&self) -> bool { true }
/// 
///     fn is_noetherian(&self) -> bool { true }
/// 
///     fn dbg<'a>(&self, value: &Self::Element, out: &mut std::fmt::Formatter<'a>) -> std::fmt::Result {
///         write!(out, "{}", **value)
///     }
/// }
/// 
/// // To use the ring through a RingStore, it is also required to implement CanonicalHom<Self>
/// // and CanonicalIso<Self>.
/// 
/// impl CanonicalHom<MyRingBase> for MyRingBase {
/// 
///     type Homomorphism = ();
/// 
///     fn has_canonical_hom(&self, _from: &MyRingBase) -> Option<()> { Some(()) }
/// 
///     fn map_in(&self, _from: &MyRingBase, el: Self::Element, _: &()) -> Self::Element { el }
/// }
/// 
/// impl CanonicalIso<MyRingBase> for MyRingBase {
/// 
///     type Isomorphism = ();
/// 
///     fn has_canonical_iso(&self, _from: &MyRingBase) -> Option<()> { Some(()) }
/// 
///     fn map_out(&self, _from: &MyRingBase, el: Self::Element, _: &()) -> Self::Element { el }
/// }
/// 
/// // A type alias for the simple, by-value RingStore.
/// 
/// pub type MyRing = RingValue<MyRingBase>;
/// 
/// impl MyRingBase {
/// 
///     pub const RING: MyRing = RingValue::new(MyRingBase);
/// }
/// 
/// let ring = MyRingBase::RING;
/// assert!(ring.eq(
///     &ring.from_z(6), 
///     &ring.mul(ring.from_z(3), ring.from_z(2))
/// ));
/// ```
/// 
pub trait RingBase {

    type Element: Clone;

    fn add_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) { self.add_assign(lhs, rhs.clone()) }
    fn add_assign(&self, lhs: &mut Self::Element, rhs: Self::Element);
    fn sub_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) { self.sub_assign(lhs, rhs.clone()) }
    fn negate_inplace(&self, lhs: &mut Self::Element);
    fn mul_assign(&self, lhs: &mut Self::Element, rhs: Self::Element);
    fn mul_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) { self.mul_assign(lhs, rhs.clone()) }
    fn zero(&self) -> Self::Element { self.from_z(0) }
    fn one(&self) -> Self::Element { self.from_z(1) }
    fn neg_one(&self) -> Self::Element { self.from_z(-1) }
    fn from_z(&self, value: i32) -> Self::Element;
    fn eq(&self, lhs: &Self::Element, rhs: &Self::Element) -> bool;
    fn is_zero(&self, value: &Self::Element) -> bool { self.eq(value, &self.zero()) }
    fn is_one(&self, value: &Self::Element) -> bool { self.eq(value, &self.one()) }
    fn is_neg_one(&self, value: &Self::Element) -> bool { self.eq(value, &self.neg_one()) }
    fn is_commutative(&self) -> bool;
    fn is_noetherian(&self) -> bool;
    fn dbg<'a>(&self, value: &Self::Element, out: &mut std::fmt::Formatter<'a>) -> std::fmt::Result;

    fn square(&self, value: Self::Element) -> Self::Element {
        self.mul(value.clone(), value)
    }

    fn negate(&self, mut value: Self::Element) -> Self::Element {
        self.negate_inplace(&mut value);
        return value;
    }
    
    fn sub_assign(&self, lhs: &mut Self::Element, mut rhs: Self::Element) {
        self.negate_inplace(&mut rhs);
        self.add_assign(lhs, rhs);
    }

    ///
    /// Computes `lhs = rhs - lhs`
    /// 
    fn sub_self_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) {
        self.negate_inplace(lhs);
        self.add_assign(lhs, rhs);
    }

    ///
    /// Computes `lhs = rhs - lhs`
    /// 
    fn sub_self_assign_ref(&self, lhs: &mut Self::Element, rhs: &Self::Element) {
        self.negate_inplace(lhs);
        self.add_assign_ref(lhs, rhs);
    }

    fn add_ref(&self, lhs: &Self::Element, rhs: &Self::Element) -> Self::Element {
        let mut result = lhs.clone();
        self.add_assign_ref(&mut result, rhs);
        return result;
    }

    fn add_ref_fst(&self, lhs: &Self::Element, mut rhs: Self::Element) -> Self::Element {
        self.add_assign_ref(&mut rhs, lhs);
        return rhs;
    }

    fn add_ref_snd(&self, mut lhs: Self::Element, rhs: &Self::Element) -> Self::Element {
        self.add_assign_ref(&mut lhs, rhs);
        return lhs;
    }

    fn add(&self, mut lhs: Self::Element, rhs: Self::Element) -> Self::Element {
        self.add_assign(&mut lhs, rhs);
        return lhs;
    }

    fn sub_ref(&self, lhs: &Self::Element, rhs: &Self::Element) -> Self::Element {
        let mut result = lhs.clone();
        self.sub_assign_ref(&mut result, rhs);
        return result;
    }

    fn sub_ref_fst(&self, lhs: &Self::Element, mut rhs: Self::Element) -> Self::Element {
        self.sub_assign_ref(&mut rhs, lhs);
        self.negate_inplace(&mut rhs);
        return rhs;
    }

    fn sub_ref_snd(&self, mut lhs: Self::Element, rhs: &Self::Element) -> Self::Element {
        self.sub_assign_ref(&mut lhs, rhs);
        return lhs;
    }

    fn sub(&self, mut lhs: Self::Element, rhs: Self::Element) -> Self::Element {
        self.sub_assign(&mut lhs, rhs);
        return lhs;
    }

    fn mul_ref(&self, lhs: &Self::Element, rhs: &Self::Element) -> Self::Element {
        let mut result = lhs.clone();
        self.mul_assign_ref(&mut result, rhs);
        return result;
    }

    fn mul_ref_fst(&self, lhs: &Self::Element, mut rhs: Self::Element) -> Self::Element {
        if self.is_commutative() {
            self.mul_assign_ref(&mut rhs, lhs);
            return rhs;
        } else {
            let mut result = lhs.clone();
            self.mul_assign(&mut result, rhs);
            return result;
        }
    }

    fn mul_ref_snd(&self, mut lhs: Self::Element, rhs: &Self::Element) -> Self::Element {
        self.mul_assign_ref(&mut lhs, rhs);
        return lhs;
    }

    fn mul(&self, mut lhs: Self::Element, rhs: Self::Element) -> Self::Element {
        self.mul_assign(&mut lhs, rhs);
        return lhs;
    }
}

macro_rules! delegate {
    (fn $name:ident (&self, $($pname:ident: $ptype:ty),*) -> $rtype:ty) => {
        fn $name (&self, $($pname: $ptype),*) -> $rtype {
            self.get_ring().$name($($pname),*)
        }
    };
    (fn $name:ident (&self) -> $rtype:ty) => {
        fn $name (&self) -> $rtype {
            self.get_ring().$name()
        }
    };
}

///
/// Basic trait for objects that store (in some sense) a ring. This can
/// be a ring-by-value, a reference to a ring, or really any object that
/// provides access to a [`RingBase`] object.
/// 
/// As opposed to [`RingBase`], which is responsible for the
/// functionality and ring operations, this trait is solely responsible for
/// the storage. The two basic implementors are [`RingValue`] and [`RingRef`],
/// which just wrap a value resp. reference to a [`RingBase`] object.
/// Building on that, every object that wraps a [`RingStore`] object can implement
/// again [`RingStore`]. This applies in particular to implementors of
/// `Deref<Target: RingStore>`, for whom there is a blanket implementation.
/// 
/// # Example
/// 
/// ```
/// # use feanor_math::ring::*;
/// # use feanor_math::primitive_int::*;
/// fn add_in_ring<R: RingStore>(ring: R, a: El<R>, b: El<R>) -> El<R> {
///     ring.add(a, b)
/// }
/// 
/// let ring: RingValue<StaticRingBase<i64>> = StaticRing::<i64>::RING;
/// assert!(ring.eq(&7, &add_in_ring(ring, 3, 4)));
/// ```
/// 
/// # What does this do?
/// 
/// We need a framework that allows nesting rings, e.g. to provide a polynomial ring
/// over a finite field - say `PolyRing<FiniteField>`. However, the simplest
/// implementation
/// ```ignore
/// struct PolyRing<BaseRing: Ring> { /* omitted */ }
/// ```
/// would have the effect that `PolyRing<FiniteField>` and `PolyRing<&FiniteField>`
/// are entirely different types. While implementing relationships between them
/// is possible, the approach does not scale well when we consider many rings and
/// multiple layers of nesting.
/// 
/// # Note for implementors
/// 
/// Generally speaking it is not recommended to overwrite any of the default-implementations
/// of ring functionality, as this is against the spirit of this trait. Instead,
/// just provide an implementation of `get_ring()` and put ring functionality in
/// a custom implementation of [`RingBase`].
/// 
pub trait RingStore {
    
    type Type: RingBase + CanonicalIso<Self::Type> + ?Sized;

    fn get_ring<'a>(&'a self) -> &'a Self::Type;

    delegate!{ fn add_assign_ref(&self, lhs: &mut El<Self>, rhs: &El<Self>) -> () }
    delegate!{ fn add_assign(&self, lhs: &mut El<Self>, rhs: El<Self>) -> () }
    delegate!{ fn sub_assign_ref(&self, lhs: &mut El<Self>, rhs: &El<Self>) -> () }
    delegate!{ fn sub_self_assign(&self, lhs: &mut El<Self>, rhs: El<Self>) -> () }
    delegate!{ fn sub_self_assign_ref(&self, lhs: &mut El<Self>, rhs: &El<Self>) -> () }
    delegate!{ fn negate_inplace(&self, lhs: &mut El<Self>) -> () }
    delegate!{ fn mul_assign(&self, lhs: &mut El<Self>, rhs: El<Self>) -> () }
    delegate!{ fn mul_assign_ref(&self, lhs: &mut El<Self>, rhs: &El<Self>) -> () }
    delegate!{ fn zero(&self) -> El<Self> }
    delegate!{ fn one(&self) -> El<Self> }
    delegate!{ fn neg_one(&self) -> El<Self> }
    delegate!{ fn from_z(&self, value: i32) -> El<Self> }
    delegate!{ fn eq(&self, lhs: &El<Self>, rhs: &El<Self>) -> bool }
    delegate!{ fn is_zero(&self, value: &El<Self>) -> bool }
    delegate!{ fn is_one(&self, value: &El<Self>) -> bool }
    delegate!{ fn is_neg_one(&self, value: &El<Self>) -> bool }
    delegate!{ fn is_commutative(&self) -> bool }
    delegate!{ fn is_noetherian(&self) -> bool }
    delegate!{ fn negate(&self, value: El<Self>) -> El<Self> }
    delegate!{ fn sub_assign(&self, lhs: &mut El<Self>, rhs: El<Self>) -> () }
    delegate!{ fn add_ref(&self, lhs: &El<Self>, rhs: &El<Self>) -> El<Self> }
    delegate!{ fn add_ref_fst(&self, lhs: &El<Self>, rhs: El<Self>) -> El<Self> }
    delegate!{ fn add_ref_snd(&self, lhs: El<Self>, rhs: &El<Self>) -> El<Self> }
    delegate!{ fn add(&self, lhs: El<Self>, rhs: El<Self>) -> El<Self> }
    delegate!{ fn sub_ref(&self, lhs: &El<Self>, rhs: &El<Self>) -> El<Self> }
    delegate!{ fn sub_ref_fst(&self, lhs: &El<Self>, rhs: El<Self>) -> El<Self> }
    delegate!{ fn sub_ref_snd(&self, lhs: El<Self>, rhs: &El<Self>) -> El<Self> }
    delegate!{ fn sub(&self, lhs: El<Self>, rhs: El<Self>) -> El<Self> }
    delegate!{ fn mul_ref(&self, lhs: &El<Self>, rhs: &El<Self>) -> El<Self> }
    delegate!{ fn mul_ref_fst(&self, lhs: &El<Self>, rhs: El<Self>) -> El<Self> }
    delegate!{ fn mul_ref_snd(&self, lhs: El<Self>, rhs: &El<Self>) -> El<Self> }
    delegate!{ fn mul(&self, lhs: El<Self>, rhs: El<Self>) -> El<Self> }
    delegate!{ fn square(&self, value: El<Self>) -> El<Self> }
    
    fn coerce<S>(&self, from: &S, el: El<S>) -> El<Self>
        where S: RingStore, Self::Type: CanonicalHom<S::Type> 
    {
        self.get_ring().map_in(from.get_ring(), el, &self.get_ring().has_canonical_hom(from.get_ring()).unwrap())
    }

    fn cast<S>(&self, to: &S, el: El<Self>) -> El<S>
        where S: RingStore, Self::Type: CanonicalIso<S::Type> 
    {
        self.get_ring().map_out(to.get_ring(), el, &self.get_ring().has_canonical_iso(to.get_ring()).unwrap())
    }

    fn sum<I>(&self, els: I) -> El<Self> 
        where I: Iterator<Item = El<Self>>
    {
        els.fold(self.zero(), |a, b| self.add(a, b))
    }

    fn prod<I>(&self, els: I) -> El<Self> 
        where I: Iterator<Item = El<Self>>
    {
        els.fold(self.one(), |a, b| self.mul(a, b))
    }

    fn base_ring<'a>(&'a self) -> &'a <Self::Type as RingExtension>::BaseRing
        where Self::Type: RingExtension
    {
        self.get_ring().base_ring()
    }

    fn from(&self, x: El<<Self::Type as RingExtension>::BaseRing>) -> El<Self>
        where Self::Type: RingExtension
    {
        self.get_ring().from(x)
    }

    fn from_ref(&self, x: &El<<Self::Type as RingExtension>::BaseRing>) -> El<Self>
        where Self::Type: RingExtension
    {
        self.get_ring().from_ref(x)
    }

    fn pow(&self, x: &El<Self>, power: usize) -> El<Self> {
        algorithms::sqr_mul::generic_abs_square_and_multiply(
            x, 
            &(power as i64), 
            StaticRing::<i64>::RING, 
            |a, 
            b| self.mul(a, b), 
            |a, b| self.mul_ref(a, b), 
            self.one()
        )
    }

    fn pow_gen<R: IntegerRingStore>(&self, x: &El<Self>, power: &El<R>, integers: R) -> El<Self> {
        algorithms::sqr_mul::generic_abs_square_and_multiply(
            x, 
            power, 
            integers, 
            |a, 
            b| self.mul(a, b), 
            |a, b| self.mul_ref(a, b), 
            self.one()
        )
    }

    fn format<'a>(&'a self, value: &'a El<Self>) -> RingElementDisplayWrapper<'a, Self> {
        RingElementDisplayWrapper { ring: self, element: value }
    }

    fn println(&self, value: &El<Self>) {
        println!("{}", self.format(value));
    }
}

pub struct RingElementDisplayWrapper<'a, R: RingStore + ?Sized> {
    ring: &'a R,
    element: &'a El<R>
}

impl<'a, R: RingStore + ?Sized> std::fmt::Display for RingElementDisplayWrapper<'a, R> {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.ring.get_ring().dbg(self.element, f)
    }
}

///
/// Trait for rings R that have a canonical homomorphism `S -> R`.
/// 
/// Which homomorphisms are considered canonical is up to implementors,
/// as long as any diagram of canonical homomorphisms commutes. In
/// other words, if there are rings `R, S` and "intermediate rings"
/// `R1, ..., Rn` resp. `R1', ..., Rm'` such that there are canonical
/// homomorphisms
/// ```text
/// S -> R1 -> R2 -> ... -> Rn -> R
/// ```
/// and
/// ```text
/// S -> R1' -> R2' -> ... -> Rm' -> R
/// ```
/// then both homomorphism chains should yield same results on same
/// inputs.
/// 
/// If the canonical homomorphism might be an isomorphism, consider also
/// implementing [`CanonicalIso`].
/// 
/// # Example
/// 
/// All integer rings support canonical homomorphisms between them.
/// ```
/// # use feanor_math::ring::*;
/// # use feanor_math::primitive_int::*;
/// # use feanor_math::rings::bigint::*;
/// let r = StaticRing::<i64>::RING;
/// let s = DefaultBigIntRing::RING;
/// // on RingBase level
/// let hom = r.get_ring().has_canonical_hom(s.get_ring()).unwrap();
/// assert_eq!(8, r.get_ring().map_in(s.get_ring(), s.from_z(8), &hom));
/// // on RingStore level
/// assert_eq!(8, r.coerce(&s, s.from_z(8)));
/// ```
/// 
pub trait CanonicalHom<S>: RingBase
    where S: RingBase + ?Sized
{
    type Homomorphism;

    fn has_canonical_hom(&self, from: &S) -> Option<Self::Homomorphism>;
    fn map_in(&self, from: &S, el: S::Element, hom: &Self::Homomorphism) -> Self::Element;
}

///
/// Trait for rings R that have a canonical isomorphism `S -> R`.
/// 
/// As for [`CanonicalHom`], it is up to implementors to decide which
/// isomorphisms are canonical, as long as each diagram that contains
/// only canonical homomorphisms, canonical isomorphisms and their inverses
/// commutes.
/// In other words, if there are rings `R, S` and "intermediate rings"
/// `R1, ..., Rn` resp. `R1', ..., Rm'` such that there are canonical
/// homomorphisms `->` or isomorphisms `<~>` connecting them - e.g. like
/// ```text
/// S -> R1 -> R2 <~> R3 <~> R4 -> ... -> Rn -> R
/// ```
/// and
/// ```text
/// S <~> R1' -> R2' -> ... -> Rm' -> R
/// ```
/// then both chains should yield same results on same inputs.
/// 
/// Hence, it would be natural if the trait were symmetrical, i.e.
///  for any implementation `R: CanonicalIso<S>` there is also an
/// implementation `S: CanonicalIso<R>`. However, because of the trait
/// impl constraints of Rust, this is unpracticable and so we only
/// require the implementation `R: CanonicalHom<S>`.
/// 
pub trait CanonicalIso<S>: CanonicalHom<S>
    where S: RingBase + ?Sized
{
    type Isomorphism;

    fn has_canonical_iso(&self, from: &S) -> Option<Self::Isomorphism>;
    fn map_out(&self, from: &S, el: Self::Element, iso: &Self::Isomorphism) -> S::Element;
}

pub trait RingExtension: RingBase {
    type BaseRing: RingStore;

    fn base_ring<'a>(&'a self) -> &'a Self::BaseRing;
    fn from(&self, x: El<Self::BaseRing>) -> Self::Element;
    
    fn from_ref(&self, x: &El<Self::BaseRing>) -> Self::Element {
        self.from(x.clone())
    }
}

pub trait HashableElRing: RingBase {

    fn hash<H: std::hash::Hasher>(&self, el: &Self::Element, h: &mut H);
}

pub trait HashableElRingStore: RingStore<Type: HashableElRing> {

    fn hash<H: std::hash::Hasher>(&self, el: &El<Self>, h: &mut H) {
        self.get_ring().hash(el, h)
    }

    fn default_hash(&self, el: &El<Self>) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(el, &mut hasher);
        return <std::collections::hash_map::DefaultHasher as std::hash::Hasher>::finish(&hasher);
    }
}

impl<R> HashableElRingStore for R
    where R: RingStore<Type: HashableElRing>
{}

pub type El<R> = <<R as RingStore>::Type as RingBase>::Element;

///
/// The most fundamental [`crate::ring::RingStore`]. It is basically
/// a no-op container, i.e. stores a [`crate::ring::RingBase`] object
/// by value, and allows accessing it.
/// 
/// # Why is this necessary?
/// 
/// In fact, that we need this trait is just the result of a technical
/// detail. We cannot implement
/// ```ignore
/// impl<R: RingBase> RingStore for R {}
/// impl<'a, R: RingStore> RingStore for &;a R {}
/// ```
/// since this might cause conflicting implementations.
/// Instead, we implement
/// ```ignore
/// impl<R: RingBase> RingStore for RingValue<R> {}
/// impl<'a, R: RingStore> RingStore for &;a R {}
/// ```
/// This causes some inconvenience, as now we cannot chain
/// [`crate::ring::RingStore`] in the case of [`crate::ring::RingValue`].
/// Furthermore, this trait will be necessary everywhere - 
/// to define a reference to a ring of type `A`, we now have to
/// write `&RingValue<A>`.
/// 
/// To simplify this, we propose to use the following simple pattern:
/// Create your ring type as
/// ```ignore
/// struct ABase { ... }
/// impl RingBase for ABase { ... } 
/// ```
/// and then provide a type alias
/// ```ignore
/// type A = RingValue<ABase>;
/// ```
/// 
#[derive(Copy, Clone)]
pub struct RingValue<R: RingBase> {
    ring: R
}

impl<R: RingBase> RingValue<R> {

    pub const fn new(value: R) -> Self {
        RingValue { ring: value }
    }
}

impl<R: RingBase + CanonicalIso<R>> RingStore for RingValue<R> {

    type Type = R;
    
    fn get_ring(&self) -> &R {
        &self.ring
    }
}

///
/// The second most basic [`crate::ring::RingStore`]. Similarly to 
/// [`crate::ring::RingValue`] it is just a no-op container.
/// 
/// # Why do we need this in addition to [`crate::ring::RingValue`]?
/// 
/// The role of `RingRef` is much more niche than the role of [`crate::ring::RingValue`].
/// However, it might happen that we want to implement [`crate::ring::RingBase`]-functions (or traits on the
/// same level, e.g. [`crate::ring::CanonicalHom`], [`crate::divisibility::DivisibilityRing`]),
/// and use more high-level techniques for that (e.g. complex algorithms, for example [`crate::algorithms::eea`]
/// or [`crate::algorithms::sqr_mul`]). In this case, we only have a reference to a [`crate::ring::RingBase`]
/// object, but require a [`crate::ring::RingStore`] object to use the algorithm.
/// 
pub struct RingRef<'a, R: RingBase + ?Sized> {
    ring: &'a R
}

impl<'a, R: RingBase + ?Sized> Clone for RingRef<'a, R> {

    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, R: RingBase + ?Sized> Copy for RingRef<'a, R> {}

impl<'a, R: RingBase + ?Sized> RingRef<'a, R> {

    pub const fn new(value: &'a R) -> Self {
        RingRef { ring: value }
    }
}

impl<'a, R: RingBase + CanonicalIso<R> + ?Sized> RingStore for RingRef<'a, R> {

    type Type = R;
    
    fn get_ring(&self) -> &R {
        self.ring
    }
}

impl<'a, S: Deref<Target: RingStore>> RingStore for S {
    
    type Type = <<S as Deref>::Target as RingStore>::Type;
    
    fn get_ring<'b>(&'b self) -> &'b Self::Type {
        (**self).get_ring()
    }
}

#[cfg(test)]
use std::rc::Rc;

#[test]
fn test_ring_rc_lifetimes() {
    let ring = Rc::new(StaticRing::<i32>::RING);
    let mut ring_ref = None;
    assert!(ring_ref.is_none());
    {
        ring_ref = Some(ring.get_ring());
    }
    assert!(ring.get_ring().is_commutative());
    assert!(ring_ref.is_some());
}

#[test]
fn test_internal_wrappings_dont_matter() {
    
    #[derive(Copy, Clone)]
    pub struct ABase;

    #[allow(unused)]
    #[derive(Copy, Clone)]
    pub struct BBase<R: RingStore> {
        base: R
    }

    impl RingBase for ABase {
        type Element = i32;

        fn add_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) {
            *lhs += rhs;
        }

        fn negate_inplace(&self, lhs: &mut Self::Element) {
            *lhs = -*lhs;
        }

        fn eq(&self, lhs: &Self::Element, rhs: &Self::Element) -> bool {
            *lhs == *rhs
        }

        fn is_commutative(&self) -> bool {
            true
        }

        fn is_noetherian(&self) -> bool {
            true
        }

        fn from_z(&self, value: i32) -> Self::Element {
            value
        }

        fn mul_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) {
            *lhs *= rhs;
        }

        fn dbg<'a>(&self, _: &Self::Element, _: &mut std::fmt::Formatter<'a>) -> std::fmt::Result {
            Ok(())
        }
    }

    impl CanonicalHom<ABase> for ABase {

        type Homomorphism = ();
        
        fn has_canonical_hom(&self, _: &ABase) -> Option<()> {
            Some(())
        }

        fn map_in(&self, _: &ABase, el: <ABase as RingBase>::Element, _: &()) -> Self::Element {
            el
        }
    }

    impl CanonicalIso<ABase> for ABase {
        
        type Isomorphism = ();

        fn has_canonical_iso(&self, _: &ABase) -> Option<()> {
            Some(())
        }

        fn map_out(&self, _: &ABase, el: <ABase as RingBase>::Element, _: &()) -> Self::Element {
            el
        }
    }

    impl<R: RingStore> RingBase for BBase<R> {
        type Element = i32;

        fn add_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) {
            *lhs += rhs;
        }
        fn negate_inplace(&self, lhs: &mut Self::Element) {
            *lhs = -*lhs;
        }

        fn eq(&self, lhs: &Self::Element, rhs: &Self::Element) -> bool {
            *lhs == *rhs
        }

        fn is_commutative(&self) -> bool {
            true
        }

        fn is_noetherian(&self) -> bool {
            true
        }

        fn from_z(&self, value: i32) -> Self::Element {
            value
        }

        fn mul_assign(&self, lhs: &mut Self::Element, rhs: Self::Element) {
            *lhs *= rhs;
        }

        fn dbg<'a>(&self, _: &Self::Element, _: &mut std::fmt::Formatter<'a>) -> std::fmt::Result {
            Ok(())
        }
    }

    impl<R: RingStore> CanonicalHom<ABase> for BBase<R> {

        type Homomorphism = ();

        fn has_canonical_hom(&self, _: &ABase) -> Option<()> {
            Some(())
        }

        fn map_in(&self, _: &ABase, el: <ABase as RingBase>::Element, _: &()) -> Self::Element {
            el
        }
    }

    impl<R: RingStore, S: RingStore> CanonicalHom<BBase<S>> for BBase<R> 
        where R::Type: CanonicalHom<S::Type>
    {
        type Homomorphism = ();

        fn has_canonical_hom(&self, _: &BBase<S>) -> Option<()> {
            Some(())
        }

        fn map_in(&self, _: &BBase<S>, el: <BBase<S> as RingBase>::Element, _: &()) -> Self::Element {
            el
        }
    }

    impl<R: RingStore> CanonicalIso<BBase<R>> for BBase<R> {

        type Isomorphism = ();

        fn has_canonical_iso(&self, _: &BBase<R>) -> Option<()> {
            Some(())
        }

        fn map_out(&self, _: &BBase<R>, el: <BBase<R> as RingBase>::Element, _: &()) -> Self::Element {
            el
        }
    }

    type A = RingValue<ABase>;
    type B<R> = RingValue<BBase<R>>;

    let a: A = RingValue { ring: ABase };
    let b1: B<A> = RingValue { ring: BBase { base: a } };
    let b2: B<&B<A>> = RingValue { ring: BBase { base: &b1 } };
    let b3: B<&A> = RingValue { ring: BBase { base: &a } };
    b1.coerce(&a, 0);
    b2.coerce(&a, 0);
    b2.coerce(&b1, 0);
    b2.coerce(&b3, 0);
    (&b2).coerce(&b3, 0);
    (&b2).coerce(&&&b3, 0);
}

#[cfg(test)]
pub fn test_ring_axioms<R: RingStore, I: Iterator<Item = El<R>>>(ring: R, edge_case_elements: I) {
    let elements = edge_case_elements.collect::<Vec<_>>();
    let zero = ring.zero();
    let one = ring.one();

    // check self-subtraction
    for a in &elements {
        assert!(ring.eq(&zero, &ring.sub(a.clone(), a.clone())));
    }

    // check identity elements
    for a in &elements {
        assert!(ring.eq(&a, &ring.add(a.clone(), zero.clone())));
        assert!(ring.eq(&a, &ring.mul(a.clone(), one.clone())));
    }

    // check commutativity
    for a in &elements {
        for b in &elements {
            assert!(ring.eq(
                &ring.add(a.clone(), b.clone()), 
                &ring.add(b.clone(), a.clone())
            ));
                
            if ring.is_commutative() {
                assert!(ring.eq(
                    &ring.mul(a.clone(), b.clone()), 
                    &ring.mul(b.clone(), a.clone())
                ));
            }
        }
    }

    // check associativity
    for a in &elements {
        for b in &elements {
            for c in &elements {
                assert!(ring.eq(
                    &ring.add(a.clone(), ring.add(b.clone(), c.clone())), 
                    &ring.add(ring.add(a.clone(), b.clone()), c.clone())
                ));
                assert!(ring.eq(
                    &ring.mul(a.clone(), ring.mul(b.clone(), c.clone())), 
                    &ring.mul(ring.mul(a.clone(), b.clone()), c.clone())
                ));
            }
        }
    }
    
    // check distributivity
    for a in &elements {
        for b in &elements {
            for c in &elements {
                assert!(ring.eq(
                    &ring.mul(a.clone(), ring.add(b.clone(), c.clone())), 
                    &ring.add(ring.mul(a.clone(), b.clone()), ring.mul(a.clone(), c.clone()))
                ));
                assert!(ring.eq(
                    &ring.mul(ring.add(a.clone(), b.clone()), c.clone()), 
                    &ring.add(ring.mul(a.clone(), c.clone()), ring.mul(b.clone(), c.clone()))
                ));
            }
        }
    }
}

