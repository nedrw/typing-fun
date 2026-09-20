//! 随包素材表：内容由 `build.rs` 从 `assets/materials/manifest.toml` 生成后嵌入。
//!
//! 加素材 = 往 `assets/materials/` 丢一个 txt，再在 `manifest.toml` 里补一段。
//! 清单问题（文件缺失、语言非法、id/显示名重复、正文为空、英文素材含非 ASCII）
//! 会在构建期直接失败，正文用 `include_str!` 内嵌，运行期不读文件。

include!(concat!(env!("OUT_DIR"), "/bundled_materials.rs"));

#[cfg(test)]
mod tests {
    use super::BUNDLED;
    use crate::lessons::Lang;

    /// 英文素材只能是可打印 ASCII（美式键盘敲得出来）；
    /// 中文素材允许夹英文——中文输入法下可以用英文模式或回车直接提交字母，所以不设限。
    #[test]
    fn bundled_texts_are_typeable() {
        for (id, name, lang, text) in BUNDLED {
            assert!(!text.trim().is_empty(), "{id}（{name}）是空素材");
            if *lang == Lang::En {
                assert!(
                    text.is_ascii(),
                    "{id}（{name}）含非 ASCII 字符，美式键盘上打不出来"
                );
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
