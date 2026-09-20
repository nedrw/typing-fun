//! xorshift64：确定性伪随机，够用且零依赖。

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            Some(&items[self.below(items.len())])
        }
    }
}

/// 换一个种子（开始下一次练习时调用，避免每次都是同一段素材）。
pub fn next_seed(seed: u64) -> u64 {
    seed.wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_gives_same_sequence() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..10 {
            assert_eq!(a.next(), b.next());
        }
    }

    #[test]
    fn below_stays_in_range() {
        let mut rng = Rng::new(1);
        for _ in 0..1000 {
            assert!(rng.below(7) < 7);
        }
        assert_eq!(Rng::new(1).below(1), 0);
    }

    #[test]
    fn pick_handles_empty() {
        let mut rng = Rng::new(1);
        let empty: [u8; 0] = [];
        assert!(rng.pick(&empty).is_none());
        assert_eq!(rng.pick(&[42]), Some(&42));
    }
}
