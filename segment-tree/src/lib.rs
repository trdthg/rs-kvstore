use std::fmt::{Debug, Display};

mod generic_segtree;

struct SegmentTree<T, F, const N: usize>
    where
        [T; N]: Sized,
        F: Fn(T, T) -> T

{
    buf: Vec<T>,
    f: F
}

impl <T, F, const N: usize> SegmentTree<T, F, N>
    where
        T: Ord + Default + Copy + Display + Debug,
        [T; N]: Sized,
        F: Fn(T, T) -> T {

    const A: usize = 2 * N;

    fn new(arr: &[T; N], f: F) -> Self {
        // 开一个二倍大的数组
        let mut buf = vec![T::default(); 2 * N];
        // 根据人的天性，使用后半段保存原来的数据
        buf[N..2 * N].copy_from_slice(arr);
        // 前半段为父节点门
        for i in (1..N).rev() {
            // 调用外部的f，提高拓展行
            buf[i] = f(buf[i * 2], buf[i * 2 + 1]);
        }

        for i in 0.. {
            if 2i32.pow(i) > 2 * N as i32 {
                break;
            } else {
                let tmp = if i == 0 { 0 } else { i - 1 };
                for j in 2i32.pow(tmp)..2i32.pow(i) {
                    print!("{:?} ", buf[j as usize]);
                }
            }
            println!();
        }


        SegmentTree { buf, f }
    }

    fn query(&self, (mut l, mut r): (usize, usize)) -> T {
        // 先把指向挪到🌲底层
        l += N - 1;
        r += N - 1;
        let mut ans = self.buf[l];
        while l <= r {
            println!("{} {}", self.buf[l], self.buf[r]);
            if l % 2 == 1 {
                // 左侧是左子🌲
                ans = (self.f)(ans, self.buf[l]);
                l += 1;
            }
            if r % 2 == 0 {
                // 右侧是右子🌲
                ans = (self.f)(ans, self.buf[r]);
                r -= 1;
            }
            l /= 2;
            r /= 2;
        }
        ans
    }

    pub fn update(&mut self, mut idx: usize, val: T) {
        idx += N - 1;
        self.buf[idx] = val;
        idx /= 2;
        while idx != 1 {
            self.buf[idx] = (self.f)(self.buf[idx * 2], self.buf[idx * 2 + 1]);
            idx /= 2;
        }
    }

}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn it_works() {
        let arr = [4i32, 3, 2, 8, 5, 1, 2, 1];
        let max = |i, j| <i32>::max(i, j);

        let mut seg_tree = SegmentTree::new(&arr, max);
        println!("max(arr): {}", seg_tree.query((0, 7)));

        seg_tree.update(2, 10);
        println!("max(arr): {}", seg_tree.query((0, 7)));
    }
}

