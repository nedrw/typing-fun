//! 两个打字引擎共用的类型词表。

/// 单个字符相对目标文本的状态。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CharState {
    Todo,
    Done,
    Wrong,
}

/// 实时统计快照。两个引擎字段含义一致，界面才能用同一套渲染。
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Stats {
    /// 目标文本总字符数
    pub total: usize,
    /// 游标位置（英文=已打字数，中文=输入框里的字数）
    pub cursor: usize,
    /// 正确的字符数（净），速度按它计算
    pub correct: usize,
    /// 输入总量：英文是击键次数，中文是输入字数
    pub strokes: u32,
    /// 错误量：英文是错误击键（累计），中文是当前错字数
    pub errors: u32,
    pub secs: f64,
    /// 净速度：字符（汉字）/分钟
    pub cpm: f64,
    /// 净速度：词/分钟（5 字符 = 1 词，中文场景不适用）
    pub wpm: f64,
    /// 正确率（%）
    pub accuracy: f64,
    pub finished: bool,
}

/// 字符是否算「打对了」。
///
/// 中文模式下输入法可能给出半角标点（不同输入法、不同中英标点设置），
/// 因此把常见的全角/半角标点视为等价，避免假错误。
pub fn same_char(typed: char, target: char) -> bool {
    typed == target || normalize_punct(typed) == normalize_punct(target)
}

fn normalize_punct(c: char) -> char {
    match c {
        ',' => '，',
        '.' => '。',
        '!' => '！',
        '?' => '？',
        ':' => '：',
        ';' => '；',
        '(' => '（',
        ')' => '）',
        '、' => '，',
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn punctuation_pairs_are_equivalent() {
        assert!(same_char(',', '，'));
        assert!(same_char('。', '.'));
        assert!(same_char('？', '?'));
        assert!(same_char('打', '打'));
    }

    #[test]
    fn different_characters_are_not_equivalent() {
        assert!(!same_char('打', '字'));
        assert!(!same_char('a', 'A'));
        assert!(!same_char('1', '一'));
    }
}
