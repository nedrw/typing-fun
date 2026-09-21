//! 打字素材。
//!
//! - 随包素材：正文内嵌在二进制里（见 `crate::bundled`），只读
//! - 用户素材：导入 txt / 粘贴新建，由 `crate::store` 落到 app data dir（`materials.ron`）
//!
//! 两者都是 `Material`，界面里合并成一个列表。这里的增删改都是纯函数，
//! 落盘由调用方（`app`）决定。

use serde::{Deserialize, Serialize};

use crate::bundled;
use crate::lessons::Lang;

/// 用户素材文件名（相对 app data dir）。
pub const FILE: &str = "materials.ron";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub id: String,
    pub name: String,
    pub lang: Lang,
    pub text: String,
    /// 随包分发的素材：只读、不可删
    #[serde(default)]
    pub bundled: bool,
}

impl Material {
    /// 素材字符数（用于列表里显示长度）。
    pub fn len_chars(&self) -> usize {
        self.text.chars().filter(|c| !c.is_whitespace()).count()
    }
}

/// 构建一份随包素材列表。
pub fn bundled() -> Vec<Material> {
    bundled::BUNDLED
        .iter()
        .map(|(id, name, lang, text)| Material {
            id: (*id).to_string(),
            name: (*name).to_string(),
            lang: *lang,
            text: (*text).to_string(),
            bundled: true,
        })
        .collect()
}

/// 添加用户素材：同语言下重名的会被覆盖，方便重复导入同一份 txt。
/// 随包素材不在这里（它们编译期内嵌），所以覆盖不到、也删不掉。
pub fn upsert(all: &[Material], material: Material) -> Vec<Material> {
    let mut next: Vec<Material> = all.to_vec();
    match next
        .iter_mut()
        .find(|m| m.name == material.name && m.lang == material.lang)
    {
        Some(existing) => {
            existing.text = material.text;
            existing.bundled = false;
        }
        None => next.push(material),
    }
    next
}

/// 删除一条用户素材，返回剩余。
pub fn remove(all: &[Material], id: &str) -> Vec<Material> {
    all.iter().filter(|m| m.id != id).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material(id: &str, name: &str, lang: Lang, text: &str) -> Material {
        Material {
            id: id.to_string(),
            name: name.to_string(),
            lang,
            text: text.to_string(),
            bundled: false,
        }
    }

    #[test]
    fn same_name_and_lang_overwrites_text_and_keeps_id() {
        let all = vec![material("u1", "文章", Lang::Zh, "旧内容")];
        let next = upsert(&all, material("u2", "文章", Lang::Zh, "新内容"));
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].id, "u1", "覆盖时沿用旧 id，成绩归档不受影响");
        assert_eq!(next[0].text, "新内容");
    }

    #[test]
    fn same_name_in_other_language_stays_separate() {
        let all = vec![material("u1", "sample", Lang::En, "hello")];
        let next = upsert(&all, material("u2", "sample", Lang::Zh, "你好"));
        assert_eq!(next.len(), 2);
    }

    #[test]
    fn remove_only_hits_matching_id() {
        let all = vec![
            material("u1", "a", Lang::En, "a"),
            material("u2", "b", Lang::En, "b"),
        ];
        let next = remove(&all, "u1");
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].id, "u2");
        assert_eq!(remove(&all, "不存在").len(), 2);
    }
}
