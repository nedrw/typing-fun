//! 用户数据落盘：桌面端经 Tauri 命令写 app data dir，浏览器里退回 localStorage。
//!
//! 文件统一是 `(version: N, data: ...)` 的 RON 信封；写入由桌面壳做
//! 「临时文件 + rename」，前端只负责 serde 序列化。文件缺失或损坏都回调 `None`
//! 并打 console 警告，不让坏数据把应用卡死；旧版 localStorage 数据会自动迁移。

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// 数据文件格式版本；字段有破坏性改动时 +1，并在 `parse` 里按版本迁移。
pub const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct Envelope<T> {
    version: u32,
    data: T,
}

fn window() -> Option<web_sys::Window> {
    web_sys::window()
}

/// 在 Tauri WebView 里跑（`trunk serve` 的浏览器环境里没有这个对象）。
fn is_tauri() -> bool {
    let Some(win) = window() else {
        return false;
    };
    js_sys::Reflect::has(win.as_ref(), &JsValue::from_str("__TAURI_INTERNALS__")).unwrap_or(false)
}

/// 调 Tauri 的 `invoke`——`@tauri-apps/api` 底层用的就是它，省一个 JS 依赖。
fn invoke(cmd: &str, args: &js_sys::Object) -> Option<js_sys::Promise> {
    let win = window()?;
    let internals =
        js_sys::Reflect::get(win.as_ref(), &JsValue::from_str("__TAURI_INTERNALS__")).ok()?;
    let invoke = js_sys::Reflect::get(&internals, &JsValue::from_str("invoke")).ok()?;
    let func: js_sys::Function = invoke.dyn_into().ok()?;
    let promise = func.call2(&internals, &JsValue::from_str(cmd), args).ok()?;
    promise.dyn_into().ok()
}

fn args_with(name: &str, contents: Option<&str>) -> js_sys::Object {
    let args = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&args, &JsValue::from_str("name"), &JsValue::from_str(name));
    if let Some(contents) = contents {
        let _ = js_sys::Reflect::set(
            &args,
            &JsValue::from_str("contents"),
            &JsValue::from_str(contents),
        );
    }
    args
}

/// 读取并反序列化；文件不存在、解析失败或后端出错都回调 `None`。
/// 旧版（localStorage + JSON）的数据会在首次读到时自动迁移。
pub fn load<T>(name: &str, on_done: impl FnOnce(Option<T>) + 'static)
where
    T: DeserializeOwned + Serialize + 'static,
{
    if is_tauri() {
        let Some(promise) = invoke("read_data", &args_with(name, None)) else {
            on_done(None);
            return;
        };
        let name = name.to_string();
        wasm_bindgen_futures::spawn_local(async move {
            let raw = wasm_bindgen_futures::JsFuture::from(promise)
                .await
                .ok()
                .and_then(|value| value.as_string());
            let loaded = raw.and_then(|raw| parse(&name, &raw));
            if loaded.is_some() {
                on_done(loaded);
            } else {
                on_done(migrate_legacy(&name));
            }
        });
    } else {
        let loaded = local_storage_get(name).and_then(|raw| parse(name, &raw));
        if loaded.is_some() {
            on_done(loaded);
        } else {
            on_done(migrate_legacy(name));
        }
    }
}

/// 旧版把数据以 JSON 存在 localStorage（`typing-fun.*.v1`）；
/// 读到空时按新格式写回一次，写回失败也不影响本次使用。
fn migrate_legacy<T>(name: &str) -> Option<T>
where
    T: DeserializeOwned + Serialize,
{
    let legacy_key = match name {
        "records.ron" => "typing-fun.records.v1",
        "materials.ron" => "typing-fun.materials.v1",
        _ => return None,
    };
    let raw = window()?
        .local_storage()
        .ok()
        .flatten()?
        .get_item(legacy_key)
        .ok()
        .flatten()?;
    let data: T = match serde_json::from_str(&raw) {
        Ok(data) => data,
        Err(err) => {
            console_warn(&format!("旧数据 {legacy_key} 解析失败，已跳过：{err}"));
            return None;
        }
    };
    console_warn(&format!("检测到旧版数据 {legacy_key}，已迁移到 {name}"));
    save(name, &data, |_| {});
    Some(data)
}

/// 序列化并写入；失败回调错误信息（配额、权限、磁盘……）。
pub fn save<T>(name: &str, data: &T, on_error: impl Fn(String) + 'static)
where
    T: Serialize,
{
    let envelope = Envelope {
        version: VERSION,
        data,
    };
    let text = match ron::ser::to_string_pretty(&envelope, ron::ser::PrettyConfig::new()) {
        Ok(text) => text,
        Err(err) => {
            on_error(format!("序列化 {name} 失败：{err}"));
            return;
        }
    };
    if is_tauri() {
        let name = name.to_string();
        let Some(promise) = invoke("write_data", &args_with(&name, Some(&text))) else {
            on_error(format!("调用 write_data 失败（{name}）"));
            return;
        };
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(err) = wasm_bindgen_futures::JsFuture::from(promise).await {
                on_error(format!("写入 {name} 失败：{err:?}"));
            }
        });
    } else if let Some(storage) = window().and_then(|win| win.local_storage().ok().flatten()) {
        if let Err(err) = storage.set_item(&format!("typing-fun.{name}"), &text) {
            on_error(format!("本地存储写入失败（{name}，可能容量已满）：{err:?}"));
        }
    }
}

fn parse<T: DeserializeOwned>(name: &str, raw: &str) -> Option<T> {
    match ron::from_str::<Envelope<T>>(raw) {
        Ok(Envelope { version, data }) => {
            if version > VERSION {
                console_warn(&format!(
                    "{name} 的格式版本 {version} 比当前应用的 {VERSION} 新，尝试按当前格式读取"
                ));
            }
            Some(data)
        }
        Err(err) => {
            console_warn(&format!("{name} 解析失败，已跳过：{err}"));
            None
        }
    }
}

fn local_storage_get(name: &str) -> Option<String> {
    window()?
        .local_storage()
        .ok()
        .flatten()?
        .get_item(&format!("typing-fun.{name}"))
        .ok()
        .flatten()
}

fn console_warn(message: &str) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::warn_1(&JsValue::from_str(message));
    #[cfg(not(target_arch = "wasm32"))]
    let _ = message;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_round_trips_through_ron() {
        let envelope = Envelope {
            version: VERSION,
            data: vec![1, 2, 3],
        };
        let text = ron::ser::to_string_pretty(&envelope, ron::ser::PrettyConfig::new()).unwrap();
        assert_eq!(parse::<Vec<i32>>("t.ron", &text), Some(vec![1, 2, 3]));
    }

    #[test]
    fn broken_file_is_skipped() {
        assert_eq!(parse::<Vec<i32>>("t.ron", "{ 不是 RON"), None);
    }

    #[test]
    fn newer_version_still_parses() {
        let text = ron::ser::to_string(&Envelope {
            version: VERSION + 1,
            data: 7,
        })
        .unwrap();
        assert_eq!(parse::<i32>("t.ron", &text), Some(7));
    }
}
