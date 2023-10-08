use crate::ring::*;
use crate::field::*;

struct WorkMatrix<F: FieldStore>
    where F::Type: Field
{
    // base data
    field: F,
    rows: Vec<Vec<(usize, El<F>)>>,
    zero: El<F>,
    col_permutation: Vec<usize>,
    col_permutation_inv: Vec<usize>,
    row_count: usize,
    col_count: usize,
    base_row_count: usize,
    base_col_count: usize,
    // derived data from here
    row_nonzero_entry_counts: Vec<usize>,
    col_nonzero_entry_counts: Vec<usize>,
    cols: Vec<Vec<usize>>,
}

impl<F: FieldStore> WorkMatrix<F>
    where F::Type: Field
{
    fn new<I>(field: F, row_count: usize, col_count: usize, entries: I) -> Self
        where I: Iterator<Item = (usize, usize, El<F>)>
    {
        let mut rows = Vec::new();
        let mut cols = Vec::new();
        rows.resize_with(row_count, Vec::new);
        cols.resize(col_count, Vec::new());
        
        let mut row_nonzero_entry_counts = Vec::new();
        let mut col_nonzero_entry_counts = Vec::new();
        row_nonzero_entry_counts.resize(row_count, 0);
        col_nonzero_entry_counts.resize(col_count, 0);
        let col_permutation = (0..col_count).collect();
        let col_permutation_inv = (0..col_count).collect();

        let mut result = WorkMatrix { 
            row_nonzero_entry_counts, 
            col_nonzero_entry_counts, 
            col_permutation, 
            col_permutation_inv,
            row_count,
            col_count,
            base_row_count: row_count,
            base_col_count: col_count,
            zero: field.zero(),
            field, 
            rows, 
            cols
        };

        for (i, j, e) in entries {
            assert!(!result.field.is_zero(&e));
            let global_i = result.global_col_index(i);
            let global_j = result.global_col_index(j);
            result.rows[global_i].push((global_j, e));
            result.cols[global_j].push(global_i);
            result.row_nonzero_entry_counts[global_i] += 1;
            result.col_nonzero_entry_counts[global_j] += 1;
        }

        for i in 0..row_count {
            result.rows[i].sort_by_key(|(j, _)| *j);
        }
        for j in 0..col_count {
            result.cols[j].sort_by_key(|i| *i);
        }

        return result;
    }

    fn check_invariants(&self) {
        for j in 0..self.base_col_count {
            let mut nonzero_entries_this_column = (0..self.base_row_count).filter(|i| self.rows[*i].iter().any(|(j2, _)| *j2 == j)).collect::<Vec<_>>();
            nonzero_entries_this_column.sort();
            assert_eq!(nonzero_entries_this_column, self.cols[j]);
        }

        for j in 0..self.col_count {
            let nonzero_entry_count = (0..self.row_count).filter(|i| self.rows[*i].iter().any(|(j2, _)| *j2 == j)).count();
            assert_eq!(nonzero_entry_count, self.col_nonzero_entry_counts[j]);
        }

        for i in 0..self.row_count {
            assert_eq!(self.rows[i].iter().filter(|(j, _)| (&self.col_permutation[0..self.col_count]).contains(j)).count(), self.row_nonzero_entry_counts[i]);
        }

        for j in 0..self.col_count {
            assert_eq!(j, self.col_permutation_inv[self.col_permutation[j]]);
        }
    }

    fn replace_entry_in_cols(cols: &mut Vec<Vec<usize>>, global_col: usize, old: usize, new: usize) {
        let old_index = cols[global_col].binary_search_by_key(&old, |index| *index).unwrap();
        if let Err(new_index) = cols[global_col].binary_search_by_key(&new, |index| *index) {
            if old_index == new_index {
                cols[global_col][new_index] = new;
            } else if old_index < new_index {
                for i in old_index..(new_index - 1) {
                    cols[global_col][i] = cols[global_col][i + 1];
                }
                cols[global_col][new_index - 1] = new;
            } else {
                for i in new_index..old_index {
                    cols[global_col][i + 1] = cols[global_col][i];
                }
                cols[global_col][new_index] = new;
            }
        } else {
            // do nothing
        }
    }

    fn nonzero_entry_added(cols: &mut Vec<Vec<usize>>, row_nonzero_entry_counts: &mut Vec<usize>, col_nonzero_entry_counts: &mut Vec<usize>, global_col: usize, global_row: usize) {
        let index = cols[global_col].binary_search_by_key(&global_row, |index| *index).expect_err("element present");
        cols[global_col].insert(index, global_row);
        row_nonzero_entry_counts[global_row] += 1;
        col_nonzero_entry_counts[global_col] += 1;
    }

    fn nonzero_entry_cancelled(cols: &mut Vec<Vec<usize>>, row_nonzero_entry_counts: &mut Vec<usize>, col_nonzero_entry_counts: &mut Vec<usize>, global_col: usize, global_row: usize) {
        cols[global_col].retain(|x| *x != global_row);
        row_nonzero_entry_counts[global_row] -= 1;
        col_nonzero_entry_counts[global_col] -= 1;
    }

    fn global_row_index(&self, i: usize) -> usize {
        self.row_count - i - 1
    }

    fn global_col_index(&self, j: usize) -> usize {
        self.col_count - j - 1
    }

    fn at(&self, i: usize, j: usize) -> &El<F> {
        assert!(i < self.row_count);
        assert!(j < self.col_count);
        let hard_column = self.col_permutation[self.global_col_index(j)];
        self.rows[self.global_row_index(i)].binary_search_by_key(&hard_column, |(index, _)| *index).map(|index| &self.rows[self.global_row_index(i)][index].1).unwrap_or(&self.zero)
    }

    fn swap_cols(&mut self, j1: usize, j2: usize) {
        assert!(j1 < self.col_count);
        assert!(j2 < self.col_count);
        if j1 == j2 {
            return;
        }
        self.check_invariants();
        let global1 = self.global_col_index(j1);
        let global2 = self.global_col_index(j2);
        self.col_permutation.swap(global1, global2);
        self.check_invariants();
    }

    fn swap_rows(&mut self, i1: usize, i2: usize) {
        assert!(i1 < self.row_count);
        assert!(i2 < self.row_count);
        if i1 == i2 {
            return;
        }
        self.check_invariants();
        let global1 = self.global_row_index(i1);
        let global2 = self.global_row_index(i2);
        for (j, _) in &self.rows[global1] {
            Self::replace_entry_in_cols(&mut self.cols, *j, global1, global2);
        }
        for (j, _) in &self.rows[global2] {
            Self::replace_entry_in_cols(&mut self.cols, *j, global2, global1);
        }
        self.rows.swap(global1, global2);
        self.row_nonzero_entry_counts.swap(global1, global2);
        self.check_invariants();
    }

    fn nonzero_entries_in_row(&self, i: usize) -> usize {
        assert!(i < self.row_count);
        self.row_nonzero_entry_counts[self.global_row_index(i)]
    }

    fn nonzero_entries_in_col(&self, j: usize) -> usize {
        assert!(j < self.col_count);
        self.col_nonzero_entry_counts[self.col_permutation[self.global_col_index(j)]]
    }

    fn sub_row(&mut self, dst_i: usize, src_i: usize, factor: &El<F>) {
        self.check_invariants();
        let mut new_row = Vec::new();
        let mut dst_index = 0;
        let mut src_index = 0;
        let dst_i_global = self.global_row_index(dst_i);
        let src_i_global = self.global_row_index(src_i);
        let dst = &self.rows[dst_i_global];
        let src = &self.rows[src_i_global];
        while dst_index != dst.len() || src_index != src.len() {
            let dst_j = dst.get(dst_index).map(|e| e.0).unwrap_or(usize::MAX);
            let src_j = src.get(src_index).map(|e| e.0).unwrap_or(usize::MAX);

            if dst_j == src_j {
                let new_value = self.field.sub_ref_fst(&dst[dst_index].1, self.field.mul_ref(&src[src_index].1, factor));
                if self.field.is_zero(&new_value) {
                    // cancellation occurs - we have to adjust every value that depends on the position of nonzero entries
                    Self::nonzero_entry_cancelled(&mut self.cols, &mut self.row_nonzero_entry_counts, &mut self.col_nonzero_entry_counts, src_j, dst_i_global);
                } else {
                    // no cancellation - this entry remains nonzero
                    new_row.push((dst_j, new_value));
                }
                dst_index += 1;
                src_index += 1;
            } else if dst_j < src_j {
                // we just keep this entry, thus it remains nonzero
                new_row.push((dst_j, self.field.clone_el(&dst[dst_index].1)));
                dst_index += 1;
            } else {
                // we get a new entry, thus we have to update position of nonzero entries
                Self::nonzero_entry_added(&mut self.cols, &mut self.row_nonzero_entry_counts, &mut self.col_nonzero_entry_counts, src_j, dst_i_global);
                new_row.push((src_j, self.field.negate(self.field.mul_ref(&src[src_index].1, factor))));
                src_index += 1;
            }
        }
        self.rows[dst_i_global] = new_row;
        self.check_invariants();
    }

    ///
    /// This requires that the area left of the lower right submatrix is completely zero!
    /// 
    fn into_lower_right_submatrix(mut self) -> Self {
        self.check_invariants();
        self.row_count -= 1;
        self.col_count -= 1;
        for (i, _) in &self.rows[self.row_count] {
            self.col_nonzero_entry_counts[*i] -= 1;
        }
        debug_assert!(self.cols[self.col_permutation[self.col_count]].len() == 0);
        self.check_invariants();
        return self;
    }
}

#[cfg(test)]
use crate::rings::zn::zn_static::Zn;

#[test]
fn test_sub_row() {
    let field = Zn::<17>::RING;
    let mut a = WorkMatrix::new(field, 8, 8, [
        (0, 0, 5), (1, 1, 3), (2, 2, 1), (3, 3, 16), (4, 4, 12), (5, 5, 3), (6, 6, 1), (7, 7, 6), 
        (0, 3, 8), (5, 2, 1)
    ].into_iter());

    assert_eq!(2, a.nonzero_entries_in_row(0));
    assert_eq!(1, a.nonzero_entries_in_row(1));
    assert_eq!(2, a.nonzero_entries_in_row(5));

    assert_eq!(1, a.nonzero_entries_in_col(0));
    assert_eq!(1, a.nonzero_entries_in_col(1));
    assert_eq!(2, a.nonzero_entries_in_col(2));

    a.sub_row(4, 0, &1);
    assert_eq!(3, a.nonzero_entries_in_row(4));
    
    assert_eq!(12, *a.at(4, 0));
    assert_eq!(0, *a.at(4, 1));
    assert_eq!(0, *a.at(4, 2));
    assert_eq!(9, *a.at(4, 3));
    assert_eq!(12, *a.at(4, 4));
    assert_eq!(0, *a.at(4, 5));
    assert_eq!(0, *a.at(4, 6));
    assert_eq!(0, *a.at(4, 7));

    a.sub_row(5, 2, &1);
    assert_eq!(1, a.nonzero_entries_in_row(5));

    // after this cancellation, there are only 3 off-diagonal entries - two in row 4 and one in row 0 

    assert_eq!(0, *a.at(5, 0));
    assert_eq!(0, *a.at(5, 1));
    assert_eq!(0, *a.at(5, 2));
    assert_eq!(0, *a.at(5, 3));
    assert_eq!(0, *a.at(5, 4));
    assert_eq!(3, *a.at(5, 5));
    assert_eq!(0, *a.at(5, 6));
    assert_eq!(0, *a.at(5, 7));

    assert_eq!(2, a.nonzero_entries_in_col(0));
    assert_eq!(1, a.nonzero_entries_in_col(1));
    assert_eq!(1, a.nonzero_entries_in_col(2));
    assert_eq!(3, a.nonzero_entries_in_col(3));
    assert_eq!(1, a.nonzero_entries_in_col(4));
    assert_eq!(1, a.nonzero_entries_in_col(5));
    assert_eq!(1, a.nonzero_entries_in_col(6));
    assert_eq!(1, a.nonzero_entries_in_col(7));

    let a = a.into_lower_right_submatrix();

    assert_eq!(1, a.nonzero_entries_in_row(0));
    assert_eq!(1, a.nonzero_entries_in_row(1));
    assert_eq!(1, a.nonzero_entries_in_row(2));
    assert_eq!(2, a.nonzero_entries_in_row(3));
    assert_eq!(1, a.nonzero_entries_in_row(4));
    assert_eq!(1, a.nonzero_entries_in_row(5));
    assert_eq!(1, a.nonzero_entries_in_row(6));

    assert_eq!(1, a.nonzero_entries_in_col(0));
    assert_eq!(1, a.nonzero_entries_in_col(1));
    assert_eq!(2, a.nonzero_entries_in_col(2));
    assert_eq!(1, a.nonzero_entries_in_col(3));
    assert_eq!(1, a.nonzero_entries_in_col(4));
    assert_eq!(1, a.nonzero_entries_in_col(5));
    assert_eq!(1, a.nonzero_entries_in_col(6));
}

#[test]
fn test_swap_rows() {
    let field = Zn::<17>::RING;
    // 1     7
    // 9 2
    //     3 8
    //   6   4
    let mut a = WorkMatrix::new(field, 4, 4, [
        (0, 0, 1), (1, 1, 2), (2, 2, 3), (3, 3, 4),
        (1, 0, 9), (2, 3, 8), (0, 3, 7), (3, 1, 6)
    ].into_iter());

    a.swap_cols(0, 2);
    a.swap_rows(0, 2);
    // 3     8
    //   2 9
    //     1 7
    //   6   4

    assert_eq!(2, a.nonzero_entries_in_row(0));
    assert_eq!(2, a.nonzero_entries_in_row(1));
    assert_eq!(2, a.nonzero_entries_in_row(2));
    assert_eq!(2, a.nonzero_entries_in_row(3));

    assert_eq!(1, a.nonzero_entries_in_col(0));
    assert_eq!(2, a.nonzero_entries_in_col(1));
    assert_eq!(2, a.nonzero_entries_in_col(2));
    assert_eq!(3, a.nonzero_entries_in_col(3));

    let mut a = a.into_lower_right_submatrix();

    assert_eq!(2, a.nonzero_entries_in_row(0));
    assert_eq!(2, a.nonzero_entries_in_row(1));
    assert_eq!(2, a.nonzero_entries_in_row(2));

    assert_eq!(2, a.nonzero_entries_in_col(0));
    assert_eq!(2, a.nonzero_entries_in_col(1));
    assert_eq!(2, a.nonzero_entries_in_col(2));

    a.sub_row(2, 0, &3);
    // 2 9
    //   1 7
    //   7 4

    assert_eq!(2, a.nonzero_entries_in_row(0));
    assert_eq!(2, a.nonzero_entries_in_row(1));
    assert_eq!(2, a.nonzero_entries_in_row(2));

    assert_eq!(1, a.nonzero_entries_in_col(0));
    assert_eq!(3, a.nonzero_entries_in_col(1));
    assert_eq!(2, a.nonzero_entries_in_col(2));

    let mut a = a.into_lower_right_submatrix();

    assert_eq!(2, a.nonzero_entries_in_row(0));
    assert_eq!(2, a.nonzero_entries_in_row(1));

    assert_eq!(2, a.nonzero_entries_in_col(0));
    assert_eq!(2, a.nonzero_entries_in_col(1));

    a.swap_cols(0, 1);
    a.swap_rows(0, 1);
    // 4 7
    // 7 1

    a.sub_row(1, 0, &6);
    // 4 7
    //   10

    assert_eq!(2, a.nonzero_entries_in_row(0));
    assert_eq!(1, a.nonzero_entries_in_row(1));

    assert_eq!(1, a.nonzero_entries_in_col(0));
    assert_eq!(2, a.nonzero_entries_in_col(1));

    assert_eq!(10, *a.at(1, 1));
}