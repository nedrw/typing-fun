//! 从素材里随机截取练习片段。

use crate::lessons::Lang;
use crate::rng::Rng;

/// 按内容判断素材语言：ASCII 字母多算英文，汉字多算中文。
/// 用于导入/粘贴时自动归类，避免把英文素材挂到中文课（反之亦然）。
pub fn detect_lang(text: &str) -> Lang {
    let mut ascii = 0usize;
    let mut han = 0usize;
    for c in text.chars() {
        if c.is_ascii_alphabetic() {
            ascii += 1;
        } else if ('\u{4e00}'..='\u{9fff}').contains(&c) {
            han += 1;
        }
    }
    if ascii > han {
        Lang::En
    } else {
        Lang::Zh
    }
}

/// 取一段约 `target_chars` 个字符的片段。
///
/// - 素材本身比 `target_chars` 短（或为 0）时整段返回
/// - 含空格的素材按英文处理，起点与终点对齐到词边界，避免从半个单词开始
/// - 不含空格的素材（中文）按字符切
pub fn random_segment(text: &str, target_chars: usize, seed: u64) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let chars: Vec<char> = normalized.chars().collect();
    if target_chars == 0 || chars.len() <= target_chars {
        return normalized;
    }

    let english = normalized.contains(' ');
    let mut rng = Rng::new(seed);
    let start = rng.below(chars.len() - target_chars + 1);
    let mut begin = start;
    let mut end = start + target_chars;

    if english {
        // 起点推到词首
        while begin < chars.len() && begin > 0 && chars[begin - 1] != ' ' {
            begin += 1;
        }
        // 终点补到词尾
        while end < chars.len() && chars[end] != ' ' {
            end += 1;
        }
        if begin >= end {
            // 极端情况下（片段里只有一个超长词）退回到字符切分
            begin = start;
            end = (start + target_chars).min(chars.len());
        }
        if end < chars.len() && chars[end] == ' ' {
            end += 1;
        }
    }

    chars[begin..end]
        .iter()
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EN: &str =
        "the quick brown fox jumps over the lazy dog and then runs away into the forest";
    const ZH: &str = "打字是一项技能只要每天坚持练习手指就会慢慢记住每个键的位置速度来自准确";

    #[test]
    fn short_material_is_returned_whole() {
        assert_eq!(random_segment("hello world", 100, 1), "hello world");
        assert_eq!(random_segment(ZH, 1000, 1), ZH);
    }

    #[test]
    fn chinese_segment_has_exact_length() {
        let segment = random_segment(ZH, 12, 5);
        assert_eq!(segment.chars().count(), 12);
        assert!(ZH.contains(&segment));
    }

    #[test]
    fn english_segment_keeps_words_whole() {
        let words: Vec<&str> = EN.split(' ').collect();
        for seed in 0..20 {
            let segment = random_segment(EN, 30, seed);
            assert!(!segment.is_empty(), "seed={seed} 取到空片段");
            for word in segment.split(' ') {
                assert!(words.contains(&word), "seed={seed} 出现半个单词：{word}");
            }
        }
    }

    #[test]
    fn same_seed_same_segment() {
        assert_eq!(random_segment(ZH, 15, 3), random_segment(ZH, 15, 3));
        assert_ne!(
            random_segment(ZH, 15, 3),
            random_segment(ZH, 15, 4),
            "不同种子应取到不同片段"
        );
    }

    #[test]
    fn messy_whitespace_is_normalized() {
        let segment = random_segment("  hello \n\n world\tagain  ", 100, 1);
        assert_eq!(segment, "hello world again");
    }

    #[test]
    fn language_is_detected_from_content() {
        assert_eq!(detect_lang("Practice makes perfect"), Lang::En);
        assert_eq!(detect_lang("打字是一项技能"), Lang::Zh);
        assert_eq!(detect_lang("中文 abcdefg"), Lang::En, "按多数派判断");
        assert_eq!(detect_lang("1234"), Lang::Zh, "分不出时归中文");
    }
}
