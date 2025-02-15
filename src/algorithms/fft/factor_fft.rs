use crate::ring::*;
use crate::homomorphism::*;
use crate::mempool::*;
use crate::algorithms::fft::*;
use crate::algorithms::fft::complex_fft::*;
use crate::rings::float_complex::*;
use crate::vector::subvector::*;

pub struct FFTTableGenCooleyTuckey<R, T1, T2> 
    where R: RingStore,
        T1: FFTTable<Ring = R>,
        T2: FFTTable<Ring = R>
{
    twiddle_factors: Vec<El<R>>,
    inv_twiddle_factors: Vec<El<R>>,
    left_table: T1,
    right_table: T2,
    root_of_unity: El<R>
}

impl<R, T1, T2> FFTTableGenCooleyTuckey<R, T1, T2>
    where R: RingStore,
        T1: FFTTable<Ring = R>,
        T2: FFTTable<Ring = R>
{
    pub fn new_with_pows<F>(mut root_of_unity_pows: F, left_table: T1, right_table: T2) -> Self
        where F: FnMut(i64) -> El<R>
    {
        assert!(left_table.ring().get_ring() == right_table.ring().get_ring());

        let ring = left_table.ring();
        assert!(ring.get_ring().is_approximate() || ring.eq_el(&root_of_unity_pows(right_table.len() as i64), left_table.root_of_unity()));
        assert!(ring.get_ring().is_approximate() || ring.eq_el(&root_of_unity_pows(left_table.len() as i64), right_table.root_of_unity()));

        let root_of_unity = root_of_unity_pows(1);
        let inv_twiddle_factors = Self::create_twiddle_factors(|i| root_of_unity_pows(-i), &left_table, &right_table);
        let twiddle_factors = Self::create_twiddle_factors(root_of_unity_pows, &left_table, &right_table);

        FFTTableGenCooleyTuckey {
            twiddle_factors: twiddle_factors,
            inv_twiddle_factors: inv_twiddle_factors,
            left_table: left_table, 
            right_table: right_table,
            root_of_unity: root_of_unity
        }
    }

    pub fn left_fft_table(&self) -> &T1 {
        &self.left_table
    }
    
    pub fn right_fft_table(&self) -> &T2 {
        &self.right_table
    }
    
    pub fn new(root_of_unity: El<R>, left_table: T1, right_table: T2) -> Self {
        assert!(left_table.ring().get_ring() == right_table.ring().get_ring());
        let ring = left_table.ring();
        assert!(!ring.get_ring().is_approximate());

        let len = left_table.len() * right_table.len();
        let root_of_unity_pows = |i: i64| if i >= 0 {
            ring.pow(ring.clone_el(&root_of_unity), i as usize)
        } else {
            ring.pow(ring.clone_el(&root_of_unity), (len as i64 + (i % len as i64)) as usize)
        };

        assert!(ring.eq_el(&root_of_unity_pows(right_table.len() as i64), left_table.root_of_unity()));
        assert!(ring.eq_el(&root_of_unity_pows(left_table.len() as i64), right_table.root_of_unity()));

        let inv_twiddle_factors = Self::create_twiddle_factors(|i| root_of_unity_pows(-i), &left_table, &right_table);
        let twiddle_factors = Self::create_twiddle_factors(root_of_unity_pows, &left_table, &right_table);

        FFTTableGenCooleyTuckey {
            twiddle_factors: twiddle_factors,
            inv_twiddle_factors: inv_twiddle_factors,
            left_table: left_table, 
            right_table: right_table,
            root_of_unity: root_of_unity
        }
    }

    fn create_twiddle_factors<F>(mut root_of_unity_pows: F, left_table: &T1, right_table: &T2) -> Vec<El<R>>
        where F: FnMut(i64) -> El<R>
    {
        AllocatingMemoryProvider.get_new_init(left_table.len() * right_table.len(), |i| {
            let ri = i % right_table.len();
            let li = i / right_table.len();
            return root_of_unity_pows(left_table.unordered_fft_permutation(li) as i64 * ri as i64);
        })
    }
}

impl<R, T1, T2> PartialEq for FFTTableGenCooleyTuckey<R, T1, T2>
    where R: RingStore,
        T1: FFTTable<Ring = R> + PartialEq,
        T2: FFTTable<Ring = R> + PartialEq
{
    fn eq(&self, other: &Self) -> bool {
        self.ring().get_ring() == other.ring().get_ring() &&
            self.left_table == other.left_table &&
            self.right_table == other.right_table &&
            self.ring().eq_el(self.root_of_unity(), other.root_of_unity())
    }
}

impl<R, T1, T2> FFTTable for FFTTableGenCooleyTuckey<R, T1, T2>
    where R: RingStore,
        T1: FFTTable<Ring = R>,
        T2: FFTTable<Ring = R>
{
    type Ring = R;

    fn len(&self) -> usize {
        self.left_table.len() * self.right_table.len()
    }

    fn ring(&self) -> &R {
        self.left_table.ring()
    }

    fn root_of_unity(&self) -> &El<R> {
        &self.root_of_unity
    }

    fn unordered_fft<V, S, M, H>(&self, mut values: V, memory_provider: &M, hom: &H)
        where S: ?Sized + RingBase, 
            H: Homomorphism<<Self::Ring as RingStore>::Type, S>,
            V: VectorViewMut<S::Element>,
            M: MemoryProvider<S::Element>
    {
        for i in 0..self.right_table.len() {
            let mut v = Subvector::new(&mut values).subvector(i..).stride(self.right_table.len());
            self.left_table.unordered_fft(&mut v, memory_provider, hom);
        }
        for i in 0..self.len() {
            hom.mul_assign_map_ref(values.at_mut(i), self.inv_twiddle_factors.at(i));
        }
        for i in 0..self.left_table.len() {
            let mut v = Subvector::new(&mut values).subvector((i * self.right_table.len())..((i + 1) * self.right_table.len()));
            self.right_table.unordered_fft(&mut v, memory_provider, hom);
        }
    }

    fn unordered_inv_fft<V, S, M, H>(&self, mut values: V, memory_provider: &M, hom: &H)
        where S: ?Sized + RingBase, 
            H: Homomorphism<<Self::Ring as RingStore>::Type, S>,
            V: VectorViewMut<S::Element>,
            M: MemoryProvider<S::Element>
    {
        for i in 0..self.left_table.len() {
            let mut v = Subvector::new(&mut values).subvector((i * self.right_table.len())..((i + 1) * self.right_table.len()));
            self.right_table.unordered_inv_fft(&mut v, memory_provider, hom);
        }
        for i in 0..self.len() {
            hom.mul_assign_map_ref(values.at_mut(i), self.twiddle_factors.at(i));
            debug_assert!(self.ring().get_ring().is_approximate() || self.ring().is_one(&self.ring().mul_ref(self.twiddle_factors.at(i), self.inv_twiddle_factors.at(i))));
        }
        for i in 0..self.right_table.len() {
            let mut v = Subvector::new(&mut values).subvector(i..).stride(self.right_table.len());
            self.left_table.unordered_inv_fft(&mut v, memory_provider, hom);
        }
    }

    fn unordered_fft_permutation(&self, i: usize) -> usize {
        assert!(i < self.len());
        self.left_table.unordered_fft_permutation(i / self.right_table.len()) + self.left_table.len() * self.right_table.unordered_fft_permutation(i % self.right_table.len())
    }

    fn unordered_fft_permutation_inv(&self, i: usize) -> usize {
        assert!(i < self.len());
        self.left_table.unordered_fft_permutation_inv(i % self.left_table.len()) * self.right_table.len() + self.right_table.unordered_fft_permutation_inv(i / self.left_table.len())
    }
}

impl<R, T1, T2> ErrorEstimate for FFTTableGenCooleyTuckey<R, T1, T2> 
    where R: RingStore<Type = Complex64>, 
        T1: FFTTable<Ring = R> + ErrorEstimate, 
        T2: FFTTable<Ring = R> + ErrorEstimate
{
    fn expected_absolute_error(&self, input_bound: f64, input_error: f64) -> f64 {
        let error_after_first_fft = self.left_table.expected_absolute_error(input_bound, input_error);
        let new_input_bound = self.left_table.len() as f64 * input_bound;
        let error_after_twiddling = error_after_first_fft + new_input_bound * (root_of_unity_error() + f64::EPSILON);
        return self.right_table.expected_absolute_error(new_input_bound, error_after_twiddling);
    }
}

#[cfg(test)]
use crate::rings::zn::zn_static::Zn;
#[cfg(test)]
use crate::algorithms;
#[cfg(test)]
use crate::rings::zn::zn_42;
#[cfg(test)]
use crate::default_memory_provider;

#[test]
fn test_fft_basic() {
    let ring = Zn::<97>::RING;
    let z = ring.int_hom().map(39);
    let fft = FFTTableGenCooleyTuckey::new(ring.pow(z, 16), 
        bluestein::FFTTableBluestein::new(ring, ring.pow(z, 24), ring.pow(z, 12), 2, 3),
        bluestein::FFTTableBluestein::new(ring, ring.pow(z, 16), ring.pow(z, 12), 3, 3),
    );
    let mut values = [1, 0, 0, 1, 0, 1];
    let expected = [3, 62, 63, 96, 37, 36];
    let mut permuted_expected = [0; 6];
    for i in 0..6 {
        permuted_expected[i] = expected[fft.unordered_fft_permutation(i)];
    }

    fft.unordered_fft(&mut values, &default_memory_provider!(), &ring.identity());
    assert_eq!(values, permuted_expected);
}

#[test]
fn test_fft_long() {
    let ring = Zn::<97>::RING;
    let z = ring.int_hom().map(39);
    let fft = FFTTableGenCooleyTuckey::new(ring.pow(z, 4), 
        bluestein::FFTTableBluestein::new(ring, ring.pow(z, 6), ring.pow(z, 3), 8, 5),
        bluestein::FFTTableBluestein::new(ring, ring.pow(z, 16), ring.pow(z, 12), 3, 3),
    );
    let mut values = [1, 0, 0, 1, 0, 1, 1, 1, 0, 1, 1, 0, 1, 1, 1, 2, 2, 0, 2, 0, 1, 2, 3, 4];
    let expected = [26, 0, 75, 47, 41, 31, 28, 62, 39, 93, 53, 27, 0, 54, 74, 61, 65, 81, 63, 38, 53, 94, 89, 91];
    let mut permuted_expected = [0; 24];
    for i in 0..24 {
        permuted_expected[i] = expected[fft.unordered_fft_permutation(i)];
    }

    fft.unordered_fft(&mut values, &default_memory_provider!(), &ring.identity());
    assert_eq!(values, permuted_expected);
}

#[test]
fn test_fft_unordered() {
    let ring = Zn::<1409>::RING;
    let z = algorithms::unity_root::get_prim_root_of_unity(ring, 64 * 11).unwrap();
    let fft = FFTTableGenCooleyTuckey::new(
        ring.pow(z, 4),
        cooley_tuckey::FFTTableCooleyTuckey::new(ring, ring.pow(z, 44), 4),
        bluestein::FFTTableBluestein::new(ring, ring.pow(z, 32), ring.pow(z, 22), 11, 5),
    );
    const LEN: usize = 16 * 11;
    let mut values = [0; LEN];
    for i in 0..LEN {
        values[i] = ring.int_hom().map(i as i32);
    }
    let original = values;

    fft.unordered_fft(&mut values, &default_memory_provider!(), &ring.identity());

    let mut ordered_fft = [0; LEN];
    for i in 0..LEN {
        ordered_fft[fft.unordered_fft_permutation(i)] = values[i];
    }

    fft.unordered_inv_fft(&mut values, &default_memory_provider!(), &ring.identity());
    assert_eq!(values, original);

    fft.inv_fft(&mut ordered_fft, &default_memory_provider!(), &ring.identity());
    assert_eq!(ordered_fft, original);
}


#[test]
fn test_unordered_fft_permutation_inv() {
    let ring = Zn::<1409>::RING;
    let z = algorithms::unity_root::get_prim_root_of_unity(ring, 64 * 11).unwrap();
    let fft = FFTTableGenCooleyTuckey::new(
        ring.pow(z, 4),
        cooley_tuckey::FFTTableCooleyTuckey::new(ring, ring.pow(z, 44), 4),
        bluestein::FFTTableBluestein::new(ring, ring.pow(z, 32), ring.pow(z, 22), 11, 5),
    );
    for i in 0..(16 * 11) {
        assert_eq!(fft.unordered_fft_permutation_inv(fft.unordered_fft_permutation(i)), i);
        assert_eq!(fft.unordered_fft_permutation(fft.unordered_fft_permutation_inv(i)), i);
    }
}

#[test]
fn test_inv_fft() {
    let ring = Zn::<97>::RING;
    let z = ring.int_hom().map(39);
    let fft = FFTTableGenCooleyTuckey::new(ring.pow(z, 16), 
        bluestein::FFTTableBluestein::new(ring, ring.pow(z, 24), ring.pow(z, 12), 2, 3),
        bluestein::FFTTableBluestein::new(ring, ring.pow(z, 16), ring.pow(z, 12), 3, 3),
    );
    let mut values = [3, 62, 63, 96, 37, 36];
    let expected = [1, 0, 0, 1, 0, 1];

    fft.inv_fft(&mut values, &default_memory_provider!(), &ring.identity());
    assert_eq!(values, expected);
}

#[test]
fn test_approximate_fft() {
    let CC = Complex64::RING;
    for (p, log2_n) in [(5, 3), (53, 5), (101, 8), (503, 10)] {
        let fft = FFTTableGenCooleyTuckey::new_with_pows(
            |i| CC.root_of_unity(i, (p as i64) << log2_n), 
            bluestein::FFTTableBluestein::for_complex(CC, p), 
            cooley_tuckey::FFTTableCooleyTuckey::for_complex(CC, log2_n)
        );
        let mut array = default_memory_provider!().get_new_init(p << log2_n, |i| CC.root_of_unity(i as i64, (p as i64) << log2_n));
        fft.fft(&mut array, &default_memory_provider!(), &CC.identity());
        let err = fft.expected_absolute_error(1., 0.);
        assert!(CC.is_absolute_approx_eq(array[0], CC.zero(), err));
        assert!(CC.is_absolute_approx_eq(array[1], CC.from_f64(fft.len() as f64), err));
        for i in 2..fft.len() {
            assert!(CC.is_absolute_approx_eq(array[i], CC.zero(), err));
        }
    }
}

#[bench]
fn bench_factor_fft(bencher: &mut test::Bencher) {
    let ring = zn_42::Zn::new(1602564097);
    let fastmul_ring = zn_42::ZnFastmul::new(ring);
    let embed = |x: El<zn_42::Zn>| ring.can_iso(&fastmul_ring).unwrap().map(x);
    let root_of_unity = algorithms::unity_root::get_prim_root_of_unity(&ring, 2 * 31 * 601).unwrap();
    let fft = FFTTableGenCooleyTuckey::new(
        embed(ring.pow(root_of_unity, 2)),
        bluestein::FFTTableBluestein::new(fastmul_ring, embed(ring.pow(root_of_unity, 31)), embed(algorithms::unity_root::get_prim_root_of_unity_pow2(&ring, 11).unwrap()), 601, 11),
        bluestein::FFTTableBluestein::new(fastmul_ring, embed(ring.pow(root_of_unity, 601)), embed(algorithms::unity_root::get_prim_root_of_unity_pow2(&ring, 6).unwrap()), 31, 6),
    );
    let data = (0..(31 * 601)).map(|i| ring.int_hom().map(i)).collect::<Vec<_>>();
    let mut copy = Vec::with_capacity(31 * 601);
    bencher.iter(|| {
        copy.clear();
        copy.extend(data.iter().map(|x| ring.clone_el(x)));
        fft.unordered_fft(&mut copy[..], &default_memory_provider!(), &ring.can_hom(&fastmul_ring).unwrap());
        fft.unordered_inv_fft(&mut copy[..], &default_memory_provider!(), &ring.can_hom(&fastmul_ring).unwrap());
        assert_el_eq!(&ring, &copy[0], &data[0]);
    });
}
