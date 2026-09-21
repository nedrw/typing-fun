//! 练习会话：英文走击键引擎，中文走输入法引擎，双拼走按键判定引擎。
//! 界面通过统一接口读取。

use crate::cn_engine::CnEngine;
use crate::engine::{Engine, ErrorMode};
use crate::lessons::Lang;
use crate::model::{CharState, Stats};
use crate::sp_engine::{SpEngine, SpTarget};

pub enum Session {
    En(Engine),
    Zh(CnEngine),
    Sp(SpEngine),
}

impl Session {
    pub fn new(text: &str, lang: Lang, mode: ErrorMode) -> Self {
        match lang {
            Lang::En => Self::En(Engine::new(text, mode)),
            Lang::Zh => Self::Zh(CnEngine::new(text)),
        }
    }

    /// 双拼按键判定：目标汉字按小鹤码逐键打。
    pub fn shuangpin(text: &str, mode: ErrorMode) -> Self {
        Self::Sp(SpEngine::new(text, mode))
    }

    /// 英文 / 双拼：处理一次字符击键。中文模式忽略（输入由输入框驱动）。
    pub fn press_char(&mut self, ch: char, now_ms: f64) {
        match self {
            Self::En(engine) => {
                engine.press(ch, now_ms);
            }
            Self::Sp(engine) => {
                engine.press(ch, now_ms);
            }
            Self::Zh(_) => {}
        }
    }

    /// 英文 / 双拼：退格。中文模式忽略（退格由输入框自己处理）。
    pub fn backspace(&mut self) {
        match self {
            Self::En(engine) => engine.backspace(),
            Self::Sp(engine) => engine.backspace(),
            Self::Zh(_) => {}
        }
    }

    /// 中文：用输入框内容刷新。其余模式忽略。
    pub fn sync_text(&mut self, text: &str, now_ms: f64) {
        if let Self::Zh(engine) = self {
            engine.sync(text, now_ms);
        }
    }

    /// 追加目标文本（限时测试用）。
    pub fn extend_target(&mut self, text: &str) {
        match self {
            Self::En(e) => e.extend_target(text),
            Self::Zh(e) => e.extend_target(text),
            Self::Sp(e) => e.extend_target(text),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::En(e) => e.len(),
            Self::Zh(e) => e.len(),
            Self::Sp(e) => e.len(),
        }
    }

    /// 目标字符。双拼模式没有单一字符目标，请用 `sp_targets()`。
    pub fn target(&self) -> &[char] {
        match self {
            Self::En(e) => e.target(),
            Self::Zh(e) => e.target(),
            Self::Sp(_) => &[],
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::En(e) => e.is_empty(),
            Self::Zh(e) => e.is_empty(),
            Self::Sp(e) => e.is_empty(),
        }
    }

    pub fn cursor(&self) -> usize {
        match self {
            Self::En(e) => e.cursor(),
            Self::Zh(e) => e.cursor(),
            Self::Sp(e) => e.cursor(),
        }
    }

    pub fn state_at(&self, i: usize) -> CharState {
        match self {
            Self::En(e) => e.state_at(i),
            Self::Zh(e) => e.state_at(i),
            Self::Sp(e) => e.state_at(i),
        }
    }

    pub fn expected(&self) -> Option<char> {
        match self {
            Self::En(e) => e.expected(),
            Self::Zh(e) => e.expected(),
            Self::Sp(e) => e.expected(),
        }
    }

    pub fn mistake_at(&self) -> Option<usize> {
        match self {
            Self::En(e) => e.mistake_at(),
            Self::Zh(e) => e.mistake_at(),
            Self::Sp(e) => e.mistake_at(),
        }
    }

    pub fn finished(&self) -> bool {
        match self {
            Self::En(e) => e.finished(),
            Self::Zh(e) => e.finished(),
            Self::Sp(e) => e.finished(),
        }
    }

    pub fn top_error_keys(&self, n: usize) -> Vec<(char, u32)> {
        match self {
            Self::En(e) => e.top_error_keys(n),
            Self::Zh(e) => e.top_error_keys(n),
            Self::Sp(e) => e.top_error_keys(n),
        }
    }

    pub fn stats(&self, now_ms: f64) -> Stats {
        match self {
            Self::En(e) => e.stats(now_ms),
            Self::Zh(e) => e.stats(now_ms),
            Self::Sp(e) => e.stats(now_ms),
        }
    }

    /// 双拼模式的目标格（字 + 拼音 + 编码）；其余模式返回 `None`。
    pub fn sp_targets(&self) -> Option<&[SpTarget]> {
        match self {
            Self::Sp(e) => Some(e.targets()),
            _ => None,
        }
    }

    /// 双拼模式当前字的 字 / 拼音 / 编码；其余模式返回 `None`。
    pub fn sp_current(&self) -> Option<(char, &'static str, [char; 2])> {
        match self {
            Self::Sp(e) => e.current(),
            _ => None,
        }
    }
}
