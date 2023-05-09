use crate::divisibility::DivisibilityRingStore;
use crate::ring::*;
use crate::algorithms;
use crate::vector::VectorViewMut;

pub struct FFTTableBluestein<R>
    where R: RingStore
{
    m_fft_table: algorithms::cooley_tuckey::FFTTableCooleyTuckey<R>,
    ///
    /// This is the bitreverse fft of a part of the sequence b_i := z^(i^2) where
    /// z is a 2n-th root of unity.
    /// In particular, we choose the part b_i for 1 - n < i < n. Clearly, the value
    /// at a negative index i must be stored at index (i + m). The other values are
    /// irrelevant.
    /// 
    b_bitreverse_fft: Vec<El<R>>,
    /// contrary to expectations, this should be a 2n-th root of unity
    inv_root_of_unity: El<R>,
    n: usize
}

impl<R> FFTTableBluestein<R> 
    where R: DivisibilityRingStore
{
    pub fn new(ring: R, root_of_unity_2n: El<R>, root_of_unity_m: El<R>, n: usize, log2_m: usize) -> Self {
        // checks on m and root_of_unity_m are done by the FFTTableCooleyTuckey
        assert!((1 << log2_m) >= 2 * n + 1);

        let m = 1 << log2_m;
        let mut b = (0..m).map(|_| ring.zero()).collect::<Vec<_>>();
        b[0] = ring.one();
        for i in 1..n {
            b[i] = ring.pow(ring.clone(&root_of_unity_2n), i * i);
            b[m - i] = ring.clone(&b[i]);
        }
        let inv_root_of_unity = ring.pow(ring.clone(&root_of_unity_2n), 2 * n - 1);
        let m_fft_table = algorithms::cooley_tuckey::FFTTableCooleyTuckey::new(ring, root_of_unity_m, log2_m);
        m_fft_table.bitreverse_fft_inplace(&mut b);
        return FFTTableBluestein { 
            m_fft_table: m_fft_table, 
            b_bitreverse_fft: b, 
            inv_root_of_unity: inv_root_of_unity, 
            n: n
        };
    }

    ///
    /// Computes the FFT of the given values using Bluestein's algorithm.
    /// 
    /// This supports any given ring, as long as the precomputed values stored in the table are
    /// also contained in the new ring. The result is wrong however if the canonical homomorphism
    /// `R -> S` does not map the N-th root of unity to a primitive N-th root of unity.
    /// 
    /// Basically, the idea is to write an FFT of any length (e.g. prime length) as a convolution,
    /// and compute the convolution efficiently using a power-of-two FFT (e.g. with the Cooley-Tuckey 
    /// algorithm).
    /// 
    pub fn fft_base<V, W, S, const INV: bool>(&self, mut values: V, ring: S, mut buffer: W)
        where V: VectorViewMut<El<S>>, W: VectorViewMut<El<S>>, S: RingStore, S::Type: CanonicalHom<R::Type>
    {
        assert!(values.len() == self.n);
        assert!(buffer.len() == self.m_fft_table.len());

        let base_ring = self.m_fft_table.ring();

        // set buffer to the zero-padded sequence values_i * z^(-i^2/2)
        for i in 0..self.n {
            let value = if INV {
                values.at((self.n - i) % self.n)
            } else {
                values.at(i)
            };
            *buffer.at_mut(i) = ring.mul_ref_fst(
                value,
                ring.coerce(base_ring, base_ring.pow(base_ring.clone(&self.inv_root_of_unity), i * i))
            );
        }
        for i in self.n..self.m_fft_table.len() {
            *buffer.at_mut(i) = ring.zero();
        }

        // perform convoluted product with b using a power-of-two fft
        self.m_fft_table.bitreverse_fft_inplace_base(&mut buffer, &ring);
        for i in 0..self.m_fft_table.len() {
            ring.mul_assign(buffer.at_mut(i), ring.coerce(base_ring, base_ring.clone(&self.b_bitreverse_fft[i])));
        }
        self.m_fft_table.bitreverse_inv_fft_inplace_base(&mut buffer, &ring);

        // write values back, and multiply them with a twiddle factor
        for i in 0..self.n {
            *values.at_mut(i) = ring.mul_ref_fst(buffer.at(i), ring.coerce(base_ring, base_ring.pow(base_ring.clone(&self.inv_root_of_unity), i * i)));
        }

        if INV {
            // finally, scale by 1/n
            let scale = ring.coerce(&base_ring, base_ring.checked_div(&base_ring.one(), &base_ring.from_int(self.n as i32)).unwrap());
            for i in 0..values.len() {
                ring.mul_assign_ref(values.at_mut(i), &scale);
            }
        }
    }

    pub fn fft<V>(&self, values: V) 
        where V: VectorViewMut<El<R>>
    {
        let buffer = (0..self.m_fft_table.len()).map(|_| self.m_fft_table.ring().zero()).collect::<Vec<_>>();
        self.fft_base::<_, _, _, false>(values, self.m_fft_table.ring(), buffer);
    }

    pub fn inv_fft<V>(&self, values: V) 
        where V: VectorViewMut<El<R>>
    {
        let buffer = (0..self.m_fft_table.len()).map(|_| self.m_fft_table.ring().zero()).collect::<Vec<_>>();
        self.fft_base::<_, _, _, true>(values, self.m_fft_table.ring(), buffer);
    }
}

#[cfg(test)]
use crate::rings::zn::zn_static::*;

#[test]
fn test_fft_base() {
    let ring = Zn::<241>::RING;
    // a 5-th root of unity is 91 
    let fft = FFTTableBluestein::new(ring, ring.from_int(36), ring.from_int(111), 5, 4);
    let mut values = [1, 3, 2, 0, 7];
    let mut buffer = [0; 16];
    fft.fft_base::<_, _, _, false>(&mut values, ring, &mut buffer);
    let expected = [13, 137, 202, 206, 170];
    assert_eq!(expected, values);
}

#[test]
fn test_inv_fft_base() {
    let ring = Zn::<241>::RING;
    // a 5-th root of unity is 91 
    let fft = FFTTableBluestein::new(ring, ring.from_int(36), ring.from_int(111), 5, 4);
    let values = [1, 3, 2, 0, 7];
    let mut work = values;
    let mut buffer = [0; 16];
    fft.fft_base::<_, _, _, false>(&mut work, ring, &mut buffer);
    fft.fft_base::<_, _, _, true>(&mut work, ring, &mut buffer);
    assert_eq!(values, work);
}