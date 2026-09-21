//! 汉字 → 拼音（无调）：表由 `build.rs` 依据 `assets/pinyin/pinyin.txt` 生成后内嵌。
//!
//! 多音字用逗号分隔、按常用度排序；`ü` 统一写成 `v`，声调已去掉。

include!(concat!(env!("OUT_DIR"), "/pinyin_table.rs"));

/// 某个汉字的全部读音（无调，常用度从高到低）。
pub fn readings(ch: char) -> Option<&'static str> {
    HANZI
        .binary_search_by_key(&ch, |&(c, _)| c)
        .ok()
        .map(|index| HANZI[index].1)
}

/// 展示用的首选读音。
#[cfg(test)]
pub fn preferred(ch: char) -> Option<&'static str> {
    readings(ch)?.split(',').next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_chars_have_expected_readings() {
        assert_eq!(preferred('中'), Some("zhong"));
        assert_eq!(preferred('一'), Some("yi"));
        assert_eq!(preferred('〇'), Some("ling"));
        let xing = readings('行').unwrap();
        assert!(xing.split(',').any(|r| r == "xing"), "行：{xing}");
        assert!(xing.split(',').any(|r| r == "hang"), "行：{xing}");
    }

    #[test]
    fn table_is_sorted_for_binary_search() {
        assert!(HANZI.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(HANZI.len() > 20_000, "只解析出 {} 个字", HANZI.len());
    }

    #[test]
    fn readings_are_plain_lowercase_ascii() {
        for (ch, readings) in HANZI {
            assert!(
                readings.chars().all(|c| c.is_ascii_lowercase() || c == ','),
                "{ch} 的读音不是纯 ASCII：{readings}"
            );
        }
    }

    #[test]
    fn non_hanzi_has_no_reading() {
        assert_eq!(readings('a'), None);
        assert_eq!(readings('，'), None);
        assert_eq!(readings('あ'), None);
    }
}
