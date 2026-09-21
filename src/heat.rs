//! 火力条：打字蓄力、随时间衰减，满格进入爆发状态。
//!
//! - 蓄力：每个正确字符加一格，打错扣一点；打字越快，单位时间加得越多
//! - 衰减：速率随当前蓄力上升（`BASE + (MAX-BASE) * heat^CURVE`，单位 /秒），
//!   所以慢打只能停在中间，快打才能顶到满格——衰减速度与蓄力程度正相关
//! - 爆发：满格后进入特效状态，强度用最近几秒的即时速度映射，越快越猛

use std::collections::VecDeque;

/// 每个正确字符的蓄力量。
pub const GAIN: f64 = 0.09;
/// 每个错误击键的扣除量。
pub const MISTAKE_PENALTY: f64 = 0.05;
/// 衰减曲线：rate = BASE + (MAX - BASE) * heat^CURVE（单位：每秒蓄力量）。
pub const DECAY_BASE: f64 = 0.08;
pub const DECAY_MAX: f64 = 0.45;
pub const DECAY_CURVE: f64 = 1.6;
/// 爆发强度映射的即时速度区间（字/分）。
pub const POWER_MIN_CPM: f64 = 90.0;
pub const POWER_MAX_CPM: f64 = 330.0;
/// 满格进入爆发后，跌破这个值才退出（滞回，避免在 1.0 附近闪烁）。
pub const BURST_EXIT: f64 = 0.7;

/// 加/扣蓄力，结果限制在 0..=1。
pub fn charge(heat: f64, correct: usize, mistakes: usize) -> f64 {
    let delta = GAIN * correct as f64 - MISTAKE_PENALTY * mistakes as f64;
    (heat + delta).clamp(0.0, 1.0)
}

/// 经过 `dt_secs` 秒后的蓄力值。
pub fn decay(heat: f64, dt_secs: f64) -> f64 {
    let heat = heat.clamp(0.0, 1.0);
    let rate = DECAY_BASE + (DECAY_MAX - DECAY_BASE) * heat.powf(DECAY_CURVE);
    (heat - rate * dt_secs).max(0.0)
}

/// 满格判定（浮点留一点余量）。
pub fn is_full(heat: f64) -> bool {
    heat >= 1.0 - 1e-9
}

/// 即时速度（字/分）→ 爆发强度 0..=1。
pub fn burst_power(cpm: f64) -> f64 {
    ((cpm - POWER_MIN_CPM) / (POWER_MAX_CPM - POWER_MIN_CPM))
        .clamp(0.0, 1.0)
        .powf(0.8)
}

/// 最近 `window_ms` 内的正确输入速度（字/分）。
pub struct SpeedWindow {
    events: VecDeque<f64>,
    window_ms: f64,
}

impl SpeedWindow {
    pub fn new(window_ms: f64) -> Self {
        Self {
            events: VecDeque::new(),
            window_ms,
        }
    }

    /// 记一次正确输入（时间戳毫秒）。
    pub fn note(&mut self, now_ms: f64) {
        self.events.push_back(now_ms);
        self.prune(now_ms);
    }

    /// 丢掉时间窗之外的事件；每帧调用，停下来速度会自然回落。
    pub fn prune(&mut self, now_ms: f64) {
        let cutoff = now_ms - self.window_ms;
        while self.events.front().is_some_and(|&t| t < cutoff) {
            self.events.pop_front();
        }
    }

    pub fn cpm(&self) -> f64 {
        self.events.len() as f64 * 60_000.0 / self.window_ms
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charge_clamps_and_counts_mistakes() {
        assert_eq!(charge(0.0, 100, 0), 1.0);
        assert_eq!(charge(1.0, 0, 100), 0.0);
        assert!((charge(0.0, 1, 0) - GAIN).abs() < 1e-9);
        assert!((charge(0.5, 0, 1) - (0.5 - MISTAKE_PENALTY)).abs() < 1e-9);
        assert!((charge(0.2, 2, 1) - (0.2 + 2.0 * GAIN - MISTAKE_PENALTY)).abs() < 1e-9);
    }

    #[test]
    fn decay_is_monotonic_and_faster_when_hot() {
        let mut heat = 1.0;
        for _ in 0..40 {
            let next = decay(heat, 0.1);
            assert!(next <= heat);
            heat = next;
        }
        assert!(heat >= 0.0);
        let hot = 1.0 - decay(1.0, 0.1);
        let warm = 0.5 - decay(0.5, 0.1);
        let cold = 0.2 - decay(0.2, 0.1);
        assert!(hot > warm && warm > cold, "蓄力越高掉得越快");
    }

    #[test]
    fn full_bar_drains_at_max_rate() {
        assert!((1.0 - decay(1.0, 1.0) - DECAY_MAX).abs() < 1e-9);
        // 半格静止时不会掉到负值
        assert_eq!(decay(0.0, 10.0), 0.0);
    }

    #[test]
    fn burst_power_maps_speed_into_range() {
        assert_eq!(burst_power(0.0), 0.0);
        assert_eq!(burst_power(POWER_MAX_CPM + 100.0), 1.0);
        assert!(burst_power(200.0) > burst_power(120.0));
        assert!(burst_power(90.0) < 0.05);
    }

    #[test]
    fn speed_window_only_counts_recent_events() {
        let mut w = SpeedWindow::new(2_500.0);
        w.note(0.0);
        w.note(1_000.0);
        w.note(2_000.0);
        assert!((w.cpm() - 3.0 * 24.0).abs() < 1e-9);
        w.prune(3_000.0); // 0ms 那条过期
        assert!((w.cpm() - 2.0 * 24.0).abs() < 1e-9);
        w.clear();
        assert_eq!(w.cpm(), 0.0);
    }

    #[test]
    fn burst_exit_sits_below_full() {
        const { assert!(BURST_EXIT > 0.0 && BURST_EXIT < 1.0) };
        assert!(!is_full(BURST_EXIT));
    }

    #[test]
    fn equilibrium_favours_fast_typists() {
        // 慢打（2 字/秒）的蓄力速度顶不住满格衰减；快打（6 字/秒）可以。
        // 用 const 块把这条平衡关系变成编译期检查。
        const { assert!(2.0 * GAIN < DECAY_MAX) };
        const { assert!(6.0 * GAIN > DECAY_MAX) };
        const { assert!(2.0 * GAIN > DECAY_BASE) };
    }
}
