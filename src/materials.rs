//! 打字素材。
//!
//! - 随包分发的素材是 `assets/materials/` 下的 txt（清单见同目录 `manifest.txt`），
//!   启动时 fetch 进来，只读
//! - 用户素材（导入 txt / 粘贴新建）存 localStorage
//!
//! 两者都是 `Material`，界面里合并成一个列表。

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::lessons::Lang;
use crate::manifest;

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

const USER_KEY: &str = "typing-fun.materials.v1";
const BUNDLE_DIR: &str = "assets/materials";

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

/// 追加一条用户素材，返回追加后的全部用户素材。
pub fn push(material: Material) -> Vec<Material> {
    let mut all = load();
    all.push(material);
    save(&all);
    all
}

/// 删除一条用户素材，返回剩余用户素材。随包素材不在库里，调用它不会有副作用。
pub fn remove(id: &str) -> Vec<Material> {
    let all: Vec<Material> = load().into_iter().filter(|m| m.id != id).collect();
    save(&all);
    all
}

/// 读随包素材：先取清单，再逐个读 txt。读不到就返回空表（不影响用户素材）。
pub async fn load_bundled() -> Vec<Material> {
    let Some(manifest_text) = fetch_text(&format!("{BUNDLE_DIR}/manifest.txt")).await else {
        return Vec::new();
    };
    let mut materials = Vec::new();
    for entry in manifest::parse(&manifest_text) {
        let url = format!("{BUNDLE_DIR}/{}", entry.file);
        let Some(text) = fetch_text(&url).await else {
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        materials.push(Material {
            id: format!("bundled-{}", entry.file),
            name: entry.name,
            lang: entry.lang,
            text,
            bundled: true,
        });
    }
    materials
}

async fn fetch_text(url: &str) -> Option<String> {
    let response = JsFuture::from(web_sys::window()?.fetch_with_str(url))
        .await
        .ok()?;
    let response: web_sys::Response = response.dyn_into().ok()?;
    if !response.ok() {
        return None;
    }
    let text = JsFuture::from(response.text().ok()?).await.ok()?;
    text.as_string()
}
