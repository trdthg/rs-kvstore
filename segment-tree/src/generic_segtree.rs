// /// Uses feature(generic_const_exprs) for convenience; could be done with a Vec instead

// /*
// Could use in implementation:
// /// This function does not copy, simply verifies security guarentees and recasts Vec as arr.
// /// ref: https://doc.rust-lang.org/beta/src/alloc/vec/mod.rs.html#3006-3054
// fn convert_into_array<T, const N: usize> (v: Vec<T>) -> [T; N] {
//     v.try_into().unwrap_or_else(|v: Vec<T>| panic!("Vec of len {} could not be converted into array length {}", v.len(), N))
// }
// */

// pub struct SegTree<T, F, const N: usize>
// where
//     [T; 2*N]: Sized,
//     F: Fn(T, T) -> T,
// {
//     buf: [T; 2*N],
//     f: F,
// }

// /// A data structure which can respond to range queries and point updates in `O(log n)` time.
// impl<T, F, const N: usize> SegTree<T, F, N>
// where
//     T: Copy + Default /* could remove Default if I tried hard enough with iterators or used unsafe */,
//     [T; 2*N]: Sized,
//     F: Fn(T, T) -> T,
// {
//     pub fn new (arr: &[T; N], f: F) -> Self {
//         let mut buf=  [T::default(); 2*N];
//         buf[N..2*N].copy_from_slice(arr);
//         for i in (1..N).rev() {
//             // note that earlier elements are first operand
//             // we will always be evaluating them in the same order
//             // so results don't change (we do NOT have the guarentee of commutativity)
//             buf[i] = f(buf[2*i], buf[2*i + 1]);
//         }

//         SegTree {buf, f}
//     }

//     pub fn query (&self, (mut l, mut r): (usize, usize)) -> T {
//         l += N - 1; r += N - 1;
//         let (mut left_result, mut right_result) = (T::default(), T::default());
//         while l <= r {
//             if l % 2 == 1 {
//                 left_result = (self.f)(left_result, self.buf[l]); l += 1;
//             }
//             if r % 2 == 0 {
//                 right_result = (self.f)(self.buf[r], right_result); r -= 1;
//             }
//             l /= 2; r /= 2;
//         }
//         (self.f)(left_result, right_result)
//     }

//     pub fn update (&mut self, val: T, mut idx: usize) {
//         idx += N - 1;
//         self.buf[idx] = val;
//         idx /= 2;

//         while idx != 1 {
//             self.buf[idx] = (self.f)(self.buf[2*idx], self.buf[2*idx + 1]);
//             idx /= 2;
//         }
//     }
// }

// #[cfg(test)]
// mod segtree_tests {
//     use super::*;
//     use std::ops::Add;

//     #[test]
//     fn test_max_segtree () {
//         let arr = [4, 3, 2, 8, 5, 1, 2, 1];
//         let mut tree = SegTree::new(&arr, <u32>::max);
//         assert_eq!(tree.query((1, 7)), 8);
//         tree.update(9, 2);
//         assert_eq!(tree.query((1, 3)), 9);
//         tree.update(11, 4);
//         assert_eq!(tree.query((5, 7)), 5);
//     }

//     #[test]
//     fn test_sum_segtree () {
//         let arr = [4, 3, 2, 8, 5, 1, 2, 1];
//         let mut tree = SegTree::new(&arr, <u32>::add);
//         assert_eq!(tree.query((1, 8)), 26);
//         tree.update(9, 2);
//         assert_eq!(tree.query((1, 3)), 15);
//         tree.update(11, 4);
//         assert_eq!(tree.query((5, 8)), 9);
//     }
// }