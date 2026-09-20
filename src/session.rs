//! 练习会话：英文走击键引擎，中文走输入法引擎，界面通过统一接口读取。

use crate::cn_engine::CnEngine;
use crate::engine::{Engine, ErrorMode};
use crate::lessons::Lang;
use crate::model::{CharState, Stats};

pub enum Session {
    En(Engine),
    Zh(CnEngine),
}

impl Session {
    pub fn new(text: &str, lang: Lang, mode: ErrorMode) -> Self {
        match lang {
            Lang::En => Self::En(Engine::new(text, mode)),
            Lang::Zh => Self::Zh(CnEngine::new(text)),
        }
    }

    /// 英文：处理一次字符击键。中文模式忽略（输入由输入框驱动）。
    pub fn press_char(&mut self, ch: char, now_ms: f64) {
        if let Self::En(engine) = self {
            engine.press(ch, now_ms);
        }
    }

    /// 英文：退格。中文模式忽略（退格由输入框自己处理）。
    pub fn backspace(&mut self) {
        if let Self::En(engine) = self {
            engine.backspace();
        }
    }

    /// 中文：用输入框内容刷新。英文模式忽略。
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
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::En(e) => e.len(),
            Self::Zh(e) => e.len(),
        }
    }

    pub fn target(&self) -> &[char] {
        match self {
            Self::En(e) => e.target(),
            Self::Zh(e) => e.target(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::En(e) => e.is_empty(),
            Self::Zh(e) => e.is_empty(),
        }
    }

    pub fn cursor(&self) -> usize {
        match self {
            Self::En(e) => e.cursor(),
            Self::Zh(e) => e.cursor(),
        }
    }

    pub fn state_at(&self, i: usize) -> CharState {
        match self {
            Self::En(e) => e.state_at(i),
            Self::Zh(e) => e.state_at(i),
        }
    }

    pub fn expected(&self) -> Option<char> {
        match self {
            Self::En(e) => e.expected(),
            Self::Zh(e) => e.expected(),
        }
    }

    pub fn mistake_at(&self) -> Option<usize> {
        match self {
            Self::En(e) => e.mistake_at(),
            Self::Zh(e) => e.mistake_at(),
        }
    }

    pub fn finished(&self) -> bool {
        match self {
            Self::En(e) => e.finished(),
            Self::Zh(e) => e.finished(),
        }
    }

    pub fn top_error_keys(&self, n: usize) -> Vec<(char, u32)> {
        match self {
            Self::En(e) => e.top_error_keys(n),
            Self::Zh(e) => e.top_error_keys(n),
        }
    }

    pub fn stats(&self, now_ms: f64) -> Stats {
        match self {
            Self::En(e) => e.stats(now_ms),
            Self::Zh(e) => e.stats(now_ms),
        }
    }
}
