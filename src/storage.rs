//! 成绩记录：落到 localStorage（Tauri 的 WebView 也会持久化这份数据）。

use serde::{Deserialize, Serialize};

const KEY: &str = "typing-fun.records.v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub lesson_id: String,
    /// 成绩条目的显示名（课程名 / 素材练习 / 限时测试），老数据没有这个字段
    #[serde(default)]
    pub title: String,
    /// 完成时刻的时间戳（毫秒）
    pub at_ms: f64,
    pub cpm: f64,
    pub accuracy: f64,
    pub errors: u32,
    pub chars: usize,
    pub secs: f64,
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

pub fn load() -> Vec<Record> {
    storage()
        .and_then(|s| s.get_item(KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// 追加一条记录，返回追加后的全部记录。
pub fn push(record: Record) -> Vec<Record> {
    let mut all = load();
    all.push(record);
    if let Some(storage) = storage() {
        if let Ok(raw) = serde_json::to_string(&all) {
            let _ = storage.set_item(KEY, &raw);
        }
    }
    all
}

/// 某课程的最快记录。
pub fn best<'a>(all: &'a [Record], lesson_id: &str) -> Option<&'a Record> {
    all.iter()
        .filter(|r| r.lesson_id == lesson_id)
        .max_by(|a, b| a.cpm.total_cmp(&b.cpm))
}

/// 清空全部成绩。
pub fn clear() {
    if let Some(storage) = storage() {
        let _ = storage.remove_item(KEY);
    }
}
