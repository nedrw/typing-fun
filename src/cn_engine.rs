//! 中文输入法引擎：不判定击键，只判定输入法「提交」进来的文字。
//!
//! 中文的击键由输入法决定（全拼、双拼、五笔各不相同），统计击键没有意义。
//! 所以这里用缓冲区比对模型：每次输入事件后拿输入框里的整段文字与目标比对，
//! 得到正确字数、错字数、进度与速度（字/分）。

use std::collections::BTreeMap;

use crate::model::{same_char, CharState, Stats};

pub struct CnEngine {
    target: Vec<char>,
    typed: Vec<char>,
    /// 每个目标位置是否已经计过错（首次打错计一次，改对了也不收回）
    wrong_counted: Vec<bool>,
    /// 错字 -> 次数，供结算时的错误分布
    error_keys: BTreeMap<char, u32>,
    started_at: Option<f64>,
    finished_at: Option<f64>,
}

impl CnEngine {
    pub fn new(text: &str) -> Self {
        Self {
            // 中文练习文本不含空格（输入法里空格是选字键，打不出空格）
            target: text.chars().filter(|c| !c.is_whitespace()).collect(),
            typed: Vec::new(),
            wrong_counted: Vec::new(),
            error_keys: BTreeMap::new(),
            started_at: None,
            finished_at: None,
        }
    }

    /// 用输入框的当前内容刷新状态。
    pub fn sync(&mut self, text: &str, now_ms: f64) {
        if self.finished_at.is_some() {
            return;
        }
        let typed: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
        if self.started_at.is_none() && !typed.is_empty() {
            self.started_at = Some(now_ms);
        }
        self.typed = typed;
        self.count_new_errors();
        if self.typed.len() >= self.target.len() {
            self.finished_at = Some(now_ms);
        }
    }

    /// 每个位置首次打错时计一次；之后改对/改错都不再影响错误分布。
    fn count_new_errors(&mut self) {
        let limit = self.typed.len().min(self.target.len());
        if self.wrong_counted.len() < limit {
            self.wrong_counted.resize(limit, false);
        }
        for i in 0..limit {
            if self.wrong_counted[i] {
                continue;
            }
            if !same_char(self.typed[i], self.target[i]) {
                self.wrong_counted[i] = true;
                *self.error_keys.entry(self.target[i]).or_insert(0) += 1;
            }
        }
    }

    pub fn target(&self) -> &[char] {
        &self.target
    }

    pub fn len(&self) -> usize {
        self.target.len()
    }

    /// 追加目标文本（限时测试里用来延续素材）。
    pub fn extend_target(&mut self, text: &str) {
        self.target
            .extend(text.chars().filter(|c| !c.is_whitespace()));
        if self.typed.len() < self.target.len() {
            self.finished_at = None;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.target.is_empty()
    }

    /// 光标位置：对齐到目标文本范围内，多打的字不算进度。
    pub fn cursor(&self) -> usize {
        self.typed.len().min(self.target.len())
    }

    pub fn state_at(&self, i: usize) -> CharState {
        match (self.typed.get(i), self.target.get(i)) {
            (Some(a), Some(b)) if same_char(*a, *b) => CharState::Done,
            (Some(_), Some(_)) => CharState::Wrong,
            _ => CharState::Todo,
        }
    }

    /// 下一个应当输入的字。
    pub fn expected(&self) -> Option<char> {
        self.target.get(self.cursor()).copied()
    }

    pub fn mistake_at(&self) -> Option<usize> {
        None
    }

    pub fn finished(&self) -> bool {
        self.finished_at.is_some()
    }

    /// 打错的字，按累计次数倒序（首次打错计一次，改对了也保留）。
    pub fn top_error_keys(&self, n: usize) -> Vec<(char, u32)> {
        let mut v: Vec<(char, u32)> = self.error_keys.iter().map(|(&c, &n)| (c, n)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.truncate(n);
        v
    }

    pub fn stats(&self, now_ms: f64) -> Stats {
        let total = self.target.len();
        let cursor = self.cursor();
        let correct = (0..cursor)
            .filter(|&i| same_char(self.typed[i], self.target[i]))
            .count();
        let entered = self.typed.len();
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
        let accuracy = if entered > 0 {
            correct as f64 / entered as f64 * 100.0
        } else {
            100.0
        };
        Stats {
            total,
            cursor,
            correct,
            strokes: entered as u32,
            errors: entered.saturating_sub(correct) as u32,
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

    fn sync_all(e: &mut CnEngine, text: &str, now_ms: f64) {
        e.sync(text, now_ms);
    }

    #[test]
    fn typing_correct_text_finishes() {
        let mut e = CnEngine::new("打字练习");
        sync_all(&mut e, "打字", 0.0);
        assert_eq!(e.cursor(), 2);
        assert_eq!(e.expected(), Some('练'));
        assert!(!e.finished());
        sync_all(&mut e, "打字练习", 30_000.0);
        assert!(e.finished());
        let s = e.stats(30_000.0);
        assert_eq!(s.correct, 4);
        assert_eq!(s.errors, 0);
        assert!((s.cpm - 8.0).abs() < 1e-9, "4 字 / 30 秒 = 8 字/分");
        assert!((s.accuracy - 100.0).abs() < 1e-9);
    }

    #[test]
    fn wrong_characters_are_counted_not_blocking() {
        let mut e = CnEngine::new("打字练习");
        sync_all(&mut e, "打子", 0.0);
        assert_eq!(e.state_at(1), CharState::Wrong);
        let s = e.stats(0.0);
        assert_eq!(s.correct, 1);
        assert_eq!(s.errors, 1);
        assert!((s.accuracy - 50.0).abs() < 1e-9);
        assert_eq!(e.top_error_keys(1), vec![('字', 1)]);
        // 改对后当前错误消失，但错误分布是累计的（首次打错计一次）
        sync_all(&mut e, "打字", 1_000.0);
        assert_eq!(e.stats(1_000.0).errors, 0);
        assert_eq!(e.top_error_keys(1), vec![('字', 1)]);
    }

    #[test]
    fn repeated_mistakes_on_one_position_count_once() {
        let mut e = CnEngine::new("打字");
        sync_all(&mut e, "打子", 0.0);
        sync_all(&mut e, "打字", 1_000.0);
        sync_all(&mut e, "打我", 2_000.0);
        assert_eq!(e.top_error_keys(3), vec![('字', 1)], "同一位置只计一次");
    }

    #[test]
    fn extra_characters_count_as_errors_and_finish() {
        let mut e = CnEngine::new("打字");
        sync_all(&mut e, "打字练习", 1_000.0);
        assert!(e.finished());
        let s = e.stats(1_000.0);
        assert_eq!(s.cursor, 2, "进度不超过目标长度");
        assert_eq!(s.errors, 2);
    }

    #[test]
    fn timer_starts_on_first_input() {
        let mut e = CnEngine::new("打字");
        assert_eq!(e.stats(9_000.0).secs, 0.0);
        sync_all(&mut e, "打", 1_000.0);
        assert!((e.stats(4_000.0).secs - 3.0).abs() < 1e-9);
    }

    #[test]
    fn punctuation_variants_are_accepted() {
        let mut e = CnEngine::new("打字。");
        sync_all(&mut e, "打字.", 0.0);
        assert_eq!(e.stats(0.0).errors, 0);
        assert!(e.finished());
    }

    #[test]
    fn whitespace_is_ignored() {
        let mut e = CnEngine::new("打字练习");
        sync_all(&mut e, "打字 练习", 0.0);
        assert_eq!(e.stats(0.0).errors, 0);
        assert_eq!(e.expected(), None);
    }
}
