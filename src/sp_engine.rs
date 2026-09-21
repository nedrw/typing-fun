//! 双拼按键引擎：把目标汉字串转成小鹤双拼的两键序列，按击键判定。
//!
//! - 多音字接受任一读音：同一个字的多组编码都能通过，界面展示常用读音的编码
//! - 打错时记一次「期望键」（当前字的常用编码），Continue 模式自动补上期望键继续，
//!   StopOnError 模式原地卡住等正确的键
//! - 拿不到拼音或编码的字符（标点、呼读音节）直接从目标里丢掉

use std::collections::BTreeMap;

use crate::engine::{ErrorMode, KeyResult};
use crate::model::{CharState, Stats};
use crate::pinyin;
use crate::shuangpin;

/// 目标里的一格：汉字 + 展示用拼音 + 可接受的编码（第一项是常用读音）。
pub struct SpTarget {
    pub ch: char,
    pub pinyin: &'static str,
    pub codes: Vec<[char; 2]>,
}

pub struct SpEngine {
    targets: Vec<SpTarget>,
    /// 已经按对的键（每个字两键），长度决定游标与字的按键位置
    keys: Vec<char>,
    mode: ErrorMode,
    strokes: u32,
    errors: u32,
    error_keys: BTreeMap<char, u32>,
    started_at: Option<f64>,
    finished_at: Option<f64>,
    mistake_at: Option<usize>,
}

impl SpEngine {
    pub fn new(text: &str, mode: ErrorMode) -> Self {
        Self {
            targets: parse_targets(text),
            keys: Vec::new(),
            mode,
            strokes: 0,
            errors: 0,
            error_keys: BTreeMap::new(),
            started_at: None,
            finished_at: None,
            mistake_at: None,
        }
    }

    pub fn press(&mut self, key: char, now_ms: f64) -> KeyResult {
        let cursor = self.cursor();
        let Some(target) = self.targets.get(cursor) else {
            return KeyResult::Ignored;
        };
        if self.started_at.is_none() {
            self.started_at = Some(now_ms);
        }
        self.strokes += 1;

        let key_pos = self.keys.len() % 2;
        // 当前生效的读音：按了第一键就跟随所选读音，否则用常用读音
        let expected = if key_pos == 0 {
            target.codes[0][0]
        } else {
            let first = self.keys[self.keys.len() - 1];
            target
                .codes
                .iter()
                .find(|code| code[0] == first)
                .unwrap_or(&target.codes[0])[1]
        };
        let valid = if key_pos == 0 {
            target.codes.iter().any(|code| code[0] == key)
        } else {
            let first = self.keys[self.keys.len() - 1];
            target
                .codes
                .iter()
                .any(|code| code[0] == first && code[1] == key)
        };

        if valid {
            self.keys.push(key);
            self.mistake_at = None;
            if self.keys.len() == self.targets.len() * 2 {
                self.finished_at = Some(now_ms);
            }
            KeyResult::Correct
        } else {
            self.errors += 1;
            *self.error_keys.entry(expected).or_insert(0) += 1;
            self.mistake_at = Some(cursor);
            if self.mode == ErrorMode::Continue {
                // 自动补上期望键，保持节奏；错误已经计入正确率
                self.keys.push(expected);
                if self.keys.len() == self.targets.len() * 2 {
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
        self.keys.pop();
        self.mistake_at = None;
    }

    pub fn targets(&self) -> &[SpTarget] {
        &self.targets
    }

    /// 当前字的展示信息：字、常用拼音、当前生效的编码（按了第一键后跟随所选读音）。
    pub fn current(&self) -> Option<(char, &'static str, [char; 2])> {
        let target = self.targets.get(self.cursor())?;
        let code = if self.keys.len() % 2 == 1 {
            let first = self.keys[self.keys.len() - 1];
            *target
                .codes
                .iter()
                .find(|code| code[0] == first)
                .unwrap_or(&target.codes[0])
        } else {
            target.codes[0]
        };
        Some((target.ch, target.pinyin, code))
    }

    /// 下一个应当按下的键（虚拟键盘高亮用）：按了第一键就跟随所选读音。
    pub fn expected(&self) -> Option<char> {
        let target = self.targets.get(self.cursor())?;
        let key_pos = self.keys.len() % 2;
        if key_pos == 0 {
            return Some(target.codes[0][0]);
        }
        let first = *self.keys.last()?;
        Some(
            target
                .codes
                .iter()
                .find(|code| code[0] == first)
                .unwrap_or(&target.codes[0])[1],
        )
    }

    /// 已完成的字数。
    pub fn cursor(&self) -> usize {
        self.keys.len() / 2
    }

    pub fn len(&self) -> usize {
        self.targets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    pub fn finished(&self) -> bool {
        self.finished_at.is_some()
    }

    pub fn mistake_at(&self) -> Option<usize> {
        self.mistake_at
    }

    pub fn state_at(&self, i: usize) -> CharState {
        if i < self.cursor() {
            CharState::Done
        } else if self.mistake_at == Some(i) {
            CharState::Wrong
        } else {
            CharState::Todo
        }
    }

    /// 追加目标文本（限时测试里用来延续素材）。
    pub fn extend_target(&mut self, text: &str) {
        self.targets.extend(parse_targets(text));
        if self.keys.len() < self.targets.len() * 2 {
            self.finished_at = None;
        }
    }

    /// 按错的键（期望键视角），按次数倒序。
    pub fn top_error_keys(&self, n: usize) -> Vec<(char, u32)> {
        let mut v: Vec<(char, u32)> = self.error_keys.iter().map(|(&c, &n)| (c, n)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.truncate(n);
        v
    }

    pub fn stats(&self, now_ms: f64) -> Stats {
        let cursor = self.cursor();
        let secs = match self.started_at {
            None => 0.0,
            Some(start) => ((self.finished_at.unwrap_or(now_ms) - start) / 1000.0).max(0.0),
        };
        let minutes = secs / 60.0;
        let cpm = if minutes > 0.0 {
            cursor as f64 / minutes
        } else {
            0.0
        };
        let accuracy = if self.strokes > 0 {
            (self.strokes - self.errors) as f64 / self.strokes as f64 * 100.0
        } else {
            100.0
        };
        Stats {
            total: self.targets.len(),
            cursor,
            correct: cursor,
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

/// 把一段文本转成目标格：丢掉没有读音或没有编码的字符。
fn parse_targets(text: &str) -> Vec<SpTarget> {
    text.chars()
        .filter(|c| !c.is_whitespace())
        .filter_map(|ch| {
            let readings = pinyin::readings(ch)?;
            let mut codes: Vec<[char; 2]> = Vec::new();
            let mut shown: Option<&'static str> = None;
            for reading in readings.split(',') {
                let Some(code) = shuangpin::encode(reading) else {
                    continue;
                };
                if shown.is_none() {
                    shown = Some(reading);
                }
                if !codes.contains(&code) {
                    codes.push(code);
                }
            }
            Some(SpTarget {
                ch,
                pinyin: shown?,
                codes,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_char_takes_two_keys() {
        let mut e = SpEngine::new("中", ErrorMode::Continue);
        assert_eq!(e.current(), Some(('中', "zhong", ['v', 's'])));
        assert_eq!(e.expected(), Some('v'));
        assert_eq!(e.press('v', 0.0), KeyResult::Correct);
        assert_eq!(e.cursor(), 0, "按了一键还不算完成一个字");
        assert_eq!(e.expected(), Some('s'));
        assert_eq!(e.press('s', 1_000.0), KeyResult::Correct);
        assert!(e.finished());
        let stats = e.stats(60_000.0);
        assert_eq!(stats.correct, 1);
        assert!((stats.secs - 1.0).abs() < 1e-9);
        assert!((stats.cpm - 60.0).abs() < 1e-9, "1 秒 1 个字 = 60 字/分");
    }

    #[test]
    fn polyphone_accepts_any_reading() {
        // 行：xing → xk，hang → hh，heng → hg……
        let mut e = SpEngine::new("行", ErrorMode::Continue);
        assert_eq!(e.press('h', 0.0), KeyResult::Correct);
        assert_eq!(e.expected(), Some('h'), "按了 hang 的第一键，期望键跟着变");
        assert_eq!(e.press('h', 0.0), KeyResult::Correct);
        assert!(e.finished());
        assert_eq!(e.stats(0.0).errors, 0);
    }

    #[test]
    fn wrong_second_key_follows_the_chosen_reading() {
        let mut e = SpEngine::new("行", ErrorMode::Continue);
        assert_eq!(e.press('h', 0.0), KeyResult::Correct, "hang 的第一键");
        assert_eq!(e.press('k', 0.0), KeyResult::Mistake, "k 属于 xing");
        assert_eq!(
            e.top_error_keys(1),
            vec![('h', 1)],
            "期望键是 hang 的第二键"
        );
        assert!(e.finished());
    }

    #[test]
    fn continue_mode_counts_error_and_auto_corrects() {
        let mut e = SpEngine::new("中", ErrorMode::Continue);
        assert_eq!(e.press('x', 0.0), KeyResult::Mistake);
        assert_eq!(e.cursor(), 0);
        assert_eq!(e.expected(), Some('s'), "期望键 v 已被自动补上");
        assert_eq!(e.press('s', 1_000.0), KeyResult::Correct);
        assert!(e.finished());
        let stats = e.stats(60_000.0);
        assert_eq!(stats.strokes, 2, "自动补的键不算击键");
        assert_eq!(stats.errors, 1);
        assert_eq!(e.top_error_keys(1), vec![('v', 1)]);
    }

    #[test]
    fn stop_on_error_waits_for_the_right_key() {
        let mut e = SpEngine::new("中", ErrorMode::StopOnError);
        assert_eq!(e.press('x', 0.0), KeyResult::Mistake);
        assert_eq!(e.cursor(), 0);
        assert_eq!(e.expected(), Some('v'));
        assert_eq!(e.press('v', 0.0), KeyResult::Correct);
        assert_eq!(e.expected(), Some('s'));
    }

    #[test]
    fn backspace_undoes_one_key_and_finish_is_final() {
        let mut e = SpEngine::new("中", ErrorMode::Continue);
        e.press('v', 0.0);
        e.backspace();
        assert_eq!(e.expected(), Some('v'));
        e.press('v', 0.0);
        e.press('s', 0.0);
        assert!(e.finished());
        e.backspace();
        assert_eq!(e.cursor(), 1, "完成后不允许退格");
    }

    #[test]
    fn punctuation_and_unknown_chars_are_dropped() {
        let e = SpEngine::new("中，文 a！", ErrorMode::Continue);
        assert_eq!(e.len(), 2);
        assert_eq!(e.targets()[0].ch, '中');
        assert_eq!(e.targets()[1].ch, '文');
        assert_eq!(SpEngine::new("，。！", ErrorMode::Continue).len(), 0);
    }

    /// 随包中文素材与中文课程里的字都必须能按键打出来（标点会被丢掉）。
    #[test]
    fn bundled_chinese_text_is_fully_convertible() {
        use crate::bundled::BUNDLED;
        use crate::lessons::{Lang, LESSONS};

        let texts = BUNDLED
            .iter()
            .filter(|(_, _, lang, _)| *lang == Lang::Zh)
            .map(|(_, _, _, text)| (*text).to_string())
            .chain(
                LESSONS
                    .iter()
                    .filter(|lesson| lesson.lang == Lang::Zh)
                    .map(|lesson| lesson.generate(7)),
            );
        for text in texts {
            let hanzi_with_reading = text
                .chars()
                .filter(|&c| pinyin::readings(c).is_some())
                .count();
            assert_eq!(
                parse_targets(&text).len(),
                hanzi_with_reading,
                "有汉字拿不到双拼编码：{text}"
            );
        }
    }
}
