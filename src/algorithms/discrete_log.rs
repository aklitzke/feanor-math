use crate::field::Field;
use crate::primitive_int::StaticRing;
use crate::ring::*;
use crate::algorithms;
use crate::rings::finite::FiniteRing;
use crate::rings::finite::FiniteRingStore;
use crate::wrapper::RingElementWrapper;

use std::hash::Hash;
use std::collections::HashMap;

const ZZ: StaticRing<i64> = StaticRing::<i64>::RING;

///
/// Computes the discrete logarithm of value w.r.t base in the monoid given by
/// op and identity. The parameter `base_order` is only required to be a bound
/// on the size of the discrete logarithm, but in many use cases it will be the
/// order of the base in the monoid.
/// 
pub fn baby_giant_step<T, F>(value: T, base: T, base_order_bound: i64, op: F, identity: T) -> Option<i64> 
    where F: Fn(T, T) -> T, T: Clone + Hash + Eq
{
    let n = algorithms::int_bisect::root_floor(ZZ, ZZ.clone_el(&base_order_bound), 2) + 1;
    let mut giant_steps = HashMap::new();
    let giant_step = algorithms::sqr_mul::generic_abs_square_and_multiply(
        base.clone(), 
        &n, 
        &ZZ, 
        |a| op(a.clone(), a), 
        |a, b| op(a.clone(), b.clone()), 
        identity.clone()
    );
    let mut current = identity;
    for j in 0..n {
        giant_steps.insert(current.clone(), j);
        current = op(current, giant_step.clone());
    }
    current = value;
    for i in 0..n {
        if let Some(j) = giant_steps.get(&current) {
            return Some(j * n - i);
        }
        current = op(current, base.clone());
    }
    return None;
}

fn power_p_discrete_log<T, F>(value: T, p_e_base: &T, p: i64, e: usize, op: F, identity: T) -> Option<i64> 
    where F: Fn(T, T) -> T, T: Clone + Hash + Eq + std::fmt::Debug
{
    assert!(e > 0);
    assert!(algorithms::miller_rabin::is_prime(ZZ, &p, 8));

    let pow = |x: &T, e: i64| algorithms::sqr_mul::generic_abs_square_and_multiply(x.clone(), &e, ZZ, |a| op(a.clone(), a), |a, b| op(a.clone(), b.clone()), identity.clone());
    let p_base = pow(p_e_base, ZZ.pow(p, e - 1));
    debug_assert_ne!(p_base, identity);
    debug_assert_eq!(pow(&p_base, p), identity);
    let mut fill_log = 0;
    let mut current = value;
    for i in 0..e {
        let log = baby_giant_step(pow(&current, ZZ.pow(p, e - i - 1)), p_base.clone(), p, &op, identity.clone())?;
        let p_i = ZZ.pow(p, i);
        let fill = (p - log) * p_i;
        current = op(current, pow(p_e_base, fill));
        fill_log += fill;
    }
    return Some(ZZ.pow(p, e) - fill_log);
}

///
/// Computes the discrete logarithm of value w.r.t the given base in the monoid given by op and identity.
/// It is required that `order` is the order of the base element and this is finite. If the given value is
/// not contained in the submonoid generated by the base element, then None is returned.
/// 
pub fn discrete_log<T, F>(value: T, base: &T, order: i64, op: F, identity: T) -> Option<i64> 
    where F: Fn(T, T) -> T, T: Clone + Hash + Eq + std::fmt::Debug
{
    let pow = |x: &T, e: i64| algorithms::sqr_mul::generic_abs_square_and_multiply(x.clone(), &e, ZZ, |a| op(a.clone(), a), |a, b| op(a.clone(), b.clone()), identity.clone());
    debug_assert!(pow(&base, order) == identity);
    let mut current_log = 1;
    let mut current_size = 1;
    for (p, e) in algorithms::int_factor::factor(&ZZ, order) {
        let size = p.pow(e as u32);
        let power = order / &size;
        let log = power_p_discrete_log(
            pow(&value, power), 
            &pow(&base, power), 
            p,
            e, 
            &op, 
            identity.clone()
        )?;
        current_log = algorithms::eea::crt(log, current_log, &size, &current_size, ZZ);
        ZZ.mul_assign(&mut current_size, size);
    }
    return Some(current_log);
}

pub fn finite_field_log<R: FiniteRingStore>(value: El<R>, base: El<R>, Fq: R) -> Option<i64>
    where R::Type: FiniteRing + Field + HashableElRing
{
    discrete_log(RingElementWrapper::new(&Fq, value), &RingElementWrapper::new(&Fq, base), Fq.size(&StaticRing::<i64>::RING) - 1, |a, b| a * b, RingElementWrapper::new(&Fq, Fq.one()))
}

#[cfg(test)]
use crate::rings::zn::zn_static::Zn;
#[cfg(test)]
use crate::rings::zn::zn_42;
#[cfg(test)]
use crate::rings::zn::ZnRingStore;
#[cfg(test)]
use crate::homomorphism::Homomorphism;

#[test]
fn test_baby_giant_step() {
    assert_eq!(
        Some(6), 
        baby_giant_step(6, 1, 20, |a, b| a + b, 0)
    );
}

#[test]
fn test_power_p_discrete_log() {
    assert_eq!(
        Some(6), 
        power_p_discrete_log(6, &1, 3, 4, |a, b| Zn::<81>::RING.add(a, b), 0)
    );
}

#[test]
fn test_discrete_log() {
    assert_eq!(
        Some(78), 
        discrete_log(78, &1, 132, |a, b| Zn::<132>::RING.add(a, b), 0)
    );
}

#[test]
fn test_finite_field_log() {
    let Fp = zn_42::Zn::new(1009).as_field().ok().unwrap();
    assert_eq!(Some(486), finite_field_log(Fp.pow(Fp.int_hom().map(11), 486), Fp.int_hom().map(11), &Fp));
}