//! 随包素材表（纯数据，不依赖 UI/存储，可被独立编译测试）。
//!
//! 正文是 `assets/materials/*.txt`，由 `include_str!` 在编译期嵌进二进制。
//! 加素材 = 往 `assets/materials/` 丢一个 txt，再在 `BUNDLED` 里补一行；
//! 正文改动会被 cargo 的依赖追踪捕获，重新编译即可，运行时不需要读文件或发请求。

use crate::lessons::Lang;

/// `(id, 显示名, 语言, 正文)`
pub const BUNDLED: &[(&str, &str, Lang, &str)] = &[
    (
        "bundled-en-typing",
        "英文 · 打字要点",
        Lang::En,
        include_str!("../assets/materials/en-typing.txt"),
    ),
    (
        "bundled-en-numbers",
        "英文 · 数字与符号密集",
        Lang::En,
        include_str!("../assets/materials/en-numbers.txt"),
    ),
    (
        "bundled-en-pangram",
        "英文 · 字母全覆盖句子",
        Lang::En,
        include_str!("../assets/materials/en-pangram.txt"),
    ),
    (
        "bundled-zh-typing",
        "中文 · 打字要点",
        Lang::Zh,
        include_str!("../assets/materials/zh-typing.txt"),
    ),
    (
        "bundled-zh-essay",
        "中文 · 散文片段",
        Lang::Zh,
        include_str!("../assets/materials/zh-essay.txt"),
    ),
    (
        "bundled-zh-shuangpin",
        "中文 · 双拼常用词",
        Lang::Zh,
        include_str!("../assets/materials/zh-shuangpin.txt"),
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// 素材必须「打得出来」：英文素材只能有可打印 ASCII（美式键盘能敲的字符），
    /// 中文素材不能有 ASCII 字母数字（中文输入法下字母会变成拼音，打不出字母）。
    #[test]
    fn bundled_texts_are_typeable() {
        for (id, name, lang, text) in BUNDLED {
            assert!(!text.trim().is_empty(), "{id}（{name}）是空素材");
            match lang {
                Lang::En => assert!(
                    text.chars().all(|c| c.is_ascii()),
                    "{id}（{name}）含非 ASCII 字符，键盘上打不出来"
                ),
                Lang::Zh => assert!(
                    text.chars().all(|c| !c.is_ascii_alphanumeric()),
                    "{id}（{name}）含 ASCII 字母数字，中文输入法下打不出来"
                ),
            }
        }
    }

    #[test]
    fn bundled_ids_and_names_are_unique() {
        let total = BUNDLED.len();

        let mut ids: Vec<&str> = BUNDLED.iter().map(|(id, ..)| *id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "随包素材的 id 有重复");

        let mut names: Vec<&str> = BUNDLED.iter().map(|(_, name, ..)| *name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total, "随包素材的显示名有重复");
    }

    #[test]
    fn bundled_covers_both_languages() {
        for lang in [Lang::En, Lang::Zh] {
            assert!(
                BUNDLED.iter().any(|(_, _, l, _)| *l == lang),
                "{} 素材一段都没有",
                lang.name()
            );
        }
    }
}
