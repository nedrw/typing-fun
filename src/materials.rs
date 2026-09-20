//! 打字素材。
//!
//! - 随包素材：正文内嵌在二进制里（见 `crate::bundled`），只读
//! - 用户素材：导入 txt / 粘贴新建，存 localStorage
//!
//! 两者都是 `Material`，界面里合并成一个列表。

use serde::{Deserialize, Serialize};

use crate::bundled;
use crate::lessons::Lang;

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

const USER_KEY: &str = "typing-fun.materials.v1";

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// 用户自己添加的素材。
pub fn load() -> Vec<Material> {
    storage()
        .and_then(|s| s.get_item(USER_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save(all: &[Material]) {
    if let Some(storage) = storage() {
        if let Ok(raw) = serde_json::to_string(all) {
            let _ = storage.set_item(USER_KEY, &raw);
        }
    }
}

/// 添加用户素材：同语言下重名的会被覆盖，方便重复导入同一份 txt。
/// 随包素材不在这里（它们编译期内嵌），所以覆盖不到、也删不掉。
pub fn upsert(material: Material) -> Vec<Material> {
    let mut all = load();
    match all
        .iter_mut()
        .find(|m| m.name == material.name && m.lang == material.lang)
    {
        Some(existing) => {
            existing.text = material.text;
            existing.bundled = false;
        }
        None => all.push(material),
    }
    save(&all);
    all
}

/// 删除一条用户素材，返回剩余用户素材。随包素材不在库里，调用它不会有副作用。
pub fn remove(id: &str) -> Vec<Material> {
    let all: Vec<Material> = load().into_iter().filter(|m| m.id != id).collect();
    save(&all);
    all
}
