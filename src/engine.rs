//! 打字引擎：纯逻辑，不依赖 UI（可用 `rustc --test src/engine.rs` 单独跑测试）。
//!
//! 判定模型：
//! - 逐字符比对，游标随输入前进；退格可回退
//! - 计时从第一次击键开始，到最后一个字符打完结束
//! - 速度用净速度（只算正确字符），正确率用累计击键（改错不抹掉历史错误）

use std::collections::BTreeMap;

use crate::model::{CharState, Stats};

/// 打错时的处理策略。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ErrorMode {
    /// 指法练习：打错不前进，必须打对当前字符。
    StopOnError,
    /// 录入练习：打错照常前进并标红，可退格修正。
    Continue,
}

/// 一次击键的结果。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyResult {
    /// 打对
    Correct,
    /// 打错
    Mistake,
    /// 已完成或文本为空，忽略本次击键
    Ignored,
}

pub struct Engine {
    target: Vec<char>,
    /// 已输入的字符，长度即游标
    typed: Vec<char>,
    mode: ErrorMode,
    strokes: u32,
    errors: u32,
    /// 期望字符 -> 打错次数，用于结算时的「问题键」分析
    error_keys: BTreeMap<char, u32>,
    started_at: Option<f64>,
    finished_at: Option<f64>,
    /// 最近一次打错的位置，供 UI 闪红
    mistake_at: Option<usize>,
}

impl Engine {
    pub fn new(text: &str, mode: ErrorMode) -> Self {
        let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
        Self {
            target: normalized.chars().collect(),
            typed: Vec::new(),
            mode,
            strokes: 0,
            errors: 0,
            error_keys: BTreeMap::new(),
            started_at: None,
            finished_at: None,
            mistake_at: None,
        }
    }

    /// 处理一次字符击键。`now_ms` 为当前时间戳（毫秒）。
    pub fn press(&mut self, ch: char, now_ms: f64) -> KeyResult {
        let Some(&expected) = self.target.get(self.typed.len()) else {
            return KeyResult::Ignored;
        };
        if self.started_at.is_none() {
            self.started_at = Some(now_ms);
        }
        self.strokes += 1;

        if ch == expected {
            self.typed.push(ch);
            self.mistake_at = None;
            if self.typed.len() == self.target.len() {
                self.finished_at = Some(now_ms);
            }
            KeyResult::Correct
        } else {
            self.errors += 1;
            *self.error_keys.entry(expected).or_insert(0) += 1;
            self.mistake_at = Some(self.typed.len());
            if self.mode == ErrorMode::Continue {
                self.typed.push(ch);
                if self.typed.len() == self.target.len() {
                    self.finished_at = Some(now_ms);
                }
            }
            KeyResult::Mistake
        }
    }

    pub fn backspace(&mut self) {
        if self.finished_at.is_some() {
            return;
        }
        self.typed.pop();
        self.mistake_at = None;
    }

    pub fn target(&self) -> &[char] {
        &self.target
    }

    pub fn len(&self) -> usize {
        self.target.len()
    }

    /// 追加目标文本（限时测试里用来延续素材，打完一段接下一段）。
    pub fn extend_target(&mut self, text: &str) {
        if !self.target.is_empty() {
            self.target.push(' ');
        }
        let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
        self.target.extend(normalized.chars());
        if self.typed.len() < self.target.len() {
            self.finished_at = None;
        }
    }

    /// 游标位置，即已输入的字符数。
    pub fn cursor(&self) -> usize {
        self.typed.len()
    }

    pub fn is_empty(&self) -> bool {
        self.target.is_empty()
    }

    pub fn state_at(&self, i: usize) -> CharState {
        match (self.typed.get(i), self.target.get(i)) {
            (Some(a), Some(b)) if a == b => CharState::Done,
            (Some(_), Some(_)) => CharState::Wrong,
            _ => CharState::Todo,
        }
    }

    /// 下一个应当按下的字符。
    pub fn expected(&self) -> Option<char> {
        self.target.get(self.typed.len()).copied()
    }

    pub fn mistake_at(&self) -> Option<usize> {
        self.mistake_at
    }

    pub fn finished(&self) -> bool {
        self.finished_at.is_some()
    }

    /// 错误最多的键，按次数倒序。
    pub fn top_error_keys(&self, n: usize) -> Vec<(char, u32)> {
        let mut v: Vec<(char, u32)> = self.error_keys.iter().map(|(&c, &n)| (c, n)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.truncate(n);
        v
    }

    pub fn stats(&self, now_ms: f64) -> Stats {
        let cursor = self.typed.len();
        let correct = self
            .typed
            .iter()
            .zip(self.target.iter())
            .filter(|(a, b)| a == b)
            .count();
        let secs = match self.started_at {
            None => 0.0,
            Some(start) => ((self.finished_at.unwrap_or(now_ms) - start) / 1000.0).max(0.0),
        };
        let minutes = secs / 60.0;
        let cpm = if minutes > 0.0 {
            correct as f64 / minutes
        } else {
            0.0
        };
        let accuracy = if self.strokes > 0 {
            (self.strokes - self.errors) as f64 / self.strokes as f64 * 100.0
        } else {
            100.0
        };
        Stats {
            total: self.target.len(),
            cursor,
            correct,
            strokes: self.strokes,
            errors: self.errors,
            secs,
            cpm,
            wpm: cpm / 5.0,
            accuracy,
            finished: self.finished(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_on_error_blocks_cursor() {
        let mut e = Engine::new("ab", ErrorMode::StopOnError);
        assert_eq!(e.press('a', 0.0), KeyResult::Correct);
        assert_eq!(e.press('x', 10.0), KeyResult::Mistake);
        let s = e.stats(10.0);
        assert_eq!(s.cursor, 1, "打错不应前进");
        assert_eq!(s.errors, 1);
        assert_eq!(e.top_error_keys(1), vec![('b', 1)]);
        assert_eq!(e.mistake_at(), Some(1));
        assert_eq!(e.press('b', 20.0), KeyResult::Correct);
        assert!(e.finished());
    }

    #[test]
    fn timer_starts_on_first_keystroke() {
        let mut e = Engine::new("ab", ErrorMode::StopOnError);
        assert_eq!(e.stats(9_000.0).secs, 0.0, "未开始打字不应计时");
        e.press('a', 1_000.0);
        assert!((e.stats(3_500.0).secs - 2.5).abs() < 1e-9);
    }

    #[test]
    fn continue_mode_marks_wrong_char() {
        let mut e = Engine::new("abc", ErrorMode::Continue);
        e.press('a', 0.0);
        e.press('x', 0.0);
        e.press('c', 60_000.0);
        assert!(e.finished());
        assert_eq!(e.state_at(1), CharState::Wrong);
        let s = e.stats(90_000.0);
        assert_eq!(s.strokes, 3);
        assert_eq!(s.errors, 1);
        assert_eq!(s.correct, 2, "净速度只算正确字符");
        assert!((s.accuracy - 200.0 / 3.0).abs() < 1e-9);
        assert!((s.secs - 60.0).abs() < 1e-9, "计时到最后一击结束");
        assert!((s.cpm - 2.0).abs() < 1e-9);
    }

    #[test]
    fn backspace_and_finish_guard() {
        let mut e = Engine::new("ab", ErrorMode::StopOnError);
        e.press('a', 0.0);
        e.backspace();
        assert_eq!(e.stats(0.0).cursor, 0);
        e.press('a', 0.0);
        e.press('b', 0.0);
        assert!(e.finished());
        assert_eq!(e.press('b', 0.0), KeyResult::Ignored, "完成后忽略击键");
        e.backspace();
        assert_eq!(e.stats(0.0).cursor, 2, "完成后不允许退格");
    }

    #[test]
    fn whitespace_is_normalized() {
        let e = Engine::new("  a \n b\t\n", ErrorMode::StopOnError);
        assert_eq!(e.target(), &['a', ' ', 'b']);
    }
}
