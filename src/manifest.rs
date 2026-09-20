//! 素材清单（`assets/materials/manifest.txt`）的解析。
//!
//! 每行 `<语言>|<显示名>|<文件名>`，`#` 开头与空行忽略。

use crate::lessons::Lang;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Entry {
    pub lang: Lang,
    pub name: String,
    pub file: String,
}

pub fn parse(text: &str) -> Vec<Entry> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut fields = line.split('|').map(str::trim);
            let lang = match fields.next()? {
                "en" => Lang::En,
                "zh" => Lang::Zh,
                _ => return None,
            };
            let name = fields.next()?;
            let file = fields.next()?;
            if name.is_empty() || file.is_empty() {
                return None;
            }
            Some(Entry {
                lang,
                name: name.to_string(),
                file: file.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries_and_skips_noise() {
        let text = "# 注释\n\nen|英文素材|en-a.txt\nzh|中文素材|zh-a.txt\n乱写一行\nxx|未知语言|xx.txt\nen|缺字段\n";
        let entries = parse(text);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].lang, Lang::En);
        assert_eq!(entries[0].name, "英文素材");
        assert_eq!(entries[0].file, "en-a.txt");
        assert_eq!(entries[1].lang, Lang::Zh);
        assert_eq!(entries[1].file, "zh-a.txt");
    }

    #[test]
    fn trims_whitespace_around_fields() {
        let entries = parse("  en | 名字 | file.txt  ");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "名字");
        assert_eq!(entries[0].file, "file.txt");
    }

    #[test]
    fn empty_input_gives_nothing() {
        assert!(parse("").is_empty());
    }
}
