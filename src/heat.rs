//! 火力条：打字蓄力、随时间衰减，满格进入爆发状态。
//!
//! - 蓄力：每次正确输入加一格，打错扣一点；打字越快，单位时间加得越多
//! - 衰减：速率随当前蓄力上升（`base + (max-base) * heat^curve`，单位 /秒），
//!   所以慢打只能停在中间，快打才能顶到满格——衰减速度与蓄力程度正相关
//! - 爆发：满格后进入特效状态，强度用最近几秒的即时速度映射
//!
//! 中英文的速度量级差很多（英文按击键、中文输入法按整字提交、双拼一字两键），
//! 所以手感参数分成三套 [`Profile`]，否则中文永远蓄不满。

use std::collections::VecDeque;

/// 一套火力条手感参数。速度单位是「每分钟正确输入次数」：
/// 英文/双拼是键，中文输入法是字。
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    /// 每次正确输入的蓄力量
    pub gain: f64,
    /// 每次错误的扣除量
    pub mistake_penalty: f64,
    /// 衰减速率（每秒）：`base + (max - base) * heat^curve`
    pub decay_base: f64,
    pub decay_max: f64,
    pub decay_curve: f64,
    /// 爆发强度映射的即时速度区间
    pub power_min: f64,
    pub power_max: f64,
}

/// 英文（按击键）：约 300 键/分（5 键/秒）压满，120 键/分停在中段。
pub const EN: Profile = Profile {
    gain: 0.09,
    mistake_penalty: 0.05,
    decay_base: 0.08,
    decay_max: 0.45,
    decay_curve: 1.6,
    power_min: 90.0,
    power_max: 330.0,
};

/// 中文输入法（按字）：约 90 字/分（1.5 字/秒）压满，30 字/分只是小火。
pub const ZH: Profile = Profile {
    gain: 0.19,
    mistake_penalty: 0.10,
    decay_base: 0.05,
    decay_max: 0.25,
    decay_curve: 1.6,
    power_min: 30.0,
    power_max: 120.0,
};

/// 双拼按键（按击键，一字两键）：约 190 键/分（3.2 键/秒）压满。
pub const SP: Profile = Profile {
    gain: 0.10,
    mistake_penalty: 0.05,
    decay_base: 0.07,
    decay_max: 0.32,
    decay_curve: 1.6,
    power_min: 100.0,
    power_max: 260.0,
};

impl Profile {
    /// 加/扣蓄力，结果限制在 0..=1。
    pub fn charge(&self, heat: f64, correct: usize, mistakes: usize) -> f64 {
        let delta = self.gain * correct as f64 - self.mistake_penalty * mistakes as f64;
        (heat + delta).clamp(0.0, 1.0)
    }

    /// 经过 `dt_secs` 秒后的蓄力值。
    pub fn decay(&self, heat: f64, dt_secs: f64) -> f64 {
        let heat = heat.clamp(0.0, 1.0);
        let rate =
            self.decay_base + (self.decay_max - self.decay_base) * heat.powf(self.decay_curve);
        (heat - rate * dt_secs).max(0.0)
    }

    /// 即时速度（次/分）→ 爆发强度 0..=1。
    pub fn burst_power(&self, cpm: f64) -> f64 {
        ((cpm - self.power_min) / (self.power_max - self.power_min))
            .clamp(0.0, 1.0)
            .powf(0.8)
    }

    /// 给定每秒正确输入次数，静止时能稳定停住的蓄力值（0..=1）。
    /// 用来核对「多快才能压满」（测试与调参用）。
    #[cfg(test)]
    pub fn equilibrium(&self, per_sec: f64) -> f64 {
        let rate = self.gain * per_sec;
        // 端点用一点浮点余量，避免 0.09*5.0 差 1e-17 落不进“满格”
        if rate <= self.decay_base + 1e-9 {
            return 0.0;
        }
        if rate >= self.decay_max - 1e-9 {
            return 1.0;
        }
        ((rate - self.decay_base) / (self.decay_max - self.decay_base)).powf(1.0 / self.decay_curve)
    }
}

/// 满格进入爆发后，跌破这个值才退出（滞回，避免在 1.0 附近闪烁）。
pub const BURST_EXIT: f64 = 0.7;

/// 满格判定（浮点留一点余量）。
pub fn is_full(heat: f64) -> bool {
    heat >= 1.0 - 1e-9
}

/// 最近 `window_ms` 内的正确输入速度（次/分）。
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
        assert_eq!(EN.charge(0.0, 100, 0), 1.0);
        assert_eq!(EN.charge(1.0, 0, 100), 0.0);
        assert!((EN.charge(0.0, 1, 0) - EN.gain).abs() < 1e-9);
        assert!((EN.charge(0.5, 0, 1) - (0.5 - EN.mistake_penalty)).abs() < 1e-9);
        assert!((EN.charge(0.2, 2, 1) - (0.2 + 2.0 * EN.gain - EN.mistake_penalty)).abs() < 1e-9);
    }

    #[test]
    fn decay_is_monotonic_and_faster_when_hot() {
        let mut heat = 1.0;
        for _ in 0..40 {
            let next = EN.decay(heat, 0.1);
            assert!(next <= heat);
            heat = next;
        }
        assert!(heat >= 0.0);
        let hot = 1.0 - EN.decay(1.0, 0.1);
        let warm = 0.5 - EN.decay(0.5, 0.1);
        let cold = 0.2 - EN.decay(0.2, 0.1);
        assert!(hot > warm && warm > cold, "蓄力越高掉得越快");
    }

    #[test]
    fn full_bar_drains_at_max_rate() {
        for profile in [EN, ZH, SP] {
            let after = profile.decay(1.0, 1.0);
            assert!(
                (1.0 - after - profile.decay_max).abs() < 1e-9,
                "{profile:?}"
            );
            assert_eq!(profile.decay(0.0, 10.0), 0.0);
        }
    }

    #[test]
    fn profiles_match_their_typing_speeds() {
        // 英文：5 键/秒（300 键/分）压满，2 键/秒（120 键/分）停在中段
        assert_eq!(EN.equilibrium(5.0), 1.0);
        assert!((0.35..0.55).contains(&EN.equilibrium(2.0)));
        // 中文输入法：1.5 字/秒（90 字/分）压满，0.5 字/秒（30 字/分）只是小火
        assert_eq!(ZH.equilibrium(1.5), 1.0);
        assert!((0.3..0.5).contains(&ZH.equilibrium(0.5)));
        // 双拼按键：3.2 键/秒（约 190 键/分）压满，2 键/秒到不了满格
        assert_eq!(SP.equilibrium(3.2), 1.0);
        assert!(SP.equilibrium(2.0) < 0.8);
    }

    #[test]
    fn burst_power_maps_speed_into_range() {
        assert_eq!(EN.burst_power(0.0), 0.0);
        assert_eq!(EN.burst_power(EN.power_max + 100.0), 1.0);
        assert!(EN.burst_power(200.0) > EN.burst_power(120.0));
        assert!(ZH.burst_power(20.0) < 0.05);
        assert_eq!(ZH.burst_power(ZH.power_max + 100.0), 1.0);
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
}
