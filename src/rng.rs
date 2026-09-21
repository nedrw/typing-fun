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
        // 取高 32 位：xorshift64 的低位随机性较弱，抽小范围时容易有规律
        ((self.next() >> 32) as usize) % n
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
    fn below_spreads_across_buckets() {
        // 取高位的回归测试：分布不应明显偏向某个桶
        let mut rng = Rng::new(42);
        let mut counts = [0usize; 6];
        for _ in 0..6_000 {
            counts[rng.below(6)] += 1;
        }
        assert!(
            counts.iter().all(|&n| (700..=1300).contains(&n)),
            "分布太偏：{counts:?}"
        );
    }

    #[test]
    fn consecutive_seeds_visit_every_slot() {
        // 同一会话里连续换段的种子不应该总落在同一个槽
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        let mut slots = Vec::new();
        for _ in 0..24 {
            seed = next_seed(seed);
            slots.push(Rng::new(seed).below(3));
        }
        assert!(slots.contains(&0) && slots.contains(&1) && slots.contains(&2));
    }

    #[test]
    fn pick_handles_empty() {
        let mut rng = Rng::new(1);
        let empty: [u8; 0] = [];
        assert!(rng.pick(&empty).is_none());
        assert_eq!(rng.pick(&[42]), Some(&42));
    }
}
