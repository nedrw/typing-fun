//! 桌面壳：除开窗外只负责用户数据落盘。
//!
//! 前端（WASM）把 RON 文本通过 `read_data` / `write_data` 命令传来，
//! 文件写到 app data dir 下的 `data/`，写入用「临时文件 + rename」保证原子性。
//! 应用自定义命令不经过 ACL，无需额外 capability。

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use tauri::Manager;

/// 只允许字母、数字、点、下划线与短横线，避免路径穿越。
fn safe_name(name: &str) -> Result<&str, String> {
    let ok = !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if ok {
        Ok(name)
    } else {
        Err(format!("非法的数据文件名：{name:?}"))
    }
}

fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("拿不到 app data dir：{err}"))?
        .join("data");
    fs::create_dir_all(&dir).map_err(|err| format!("创建 {} 失败：{err}", dir.display()))?;
    Ok(dir)
}

/// 读一个数据文件；不存在返回 `None`。
#[tauri::command]
fn read_data(app: tauri::AppHandle, name: String) -> Result<Option<String>, String> {
    let path = data_dir(&app)?.join(safe_name(&name)?);
    match fs::read_to_string(&path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("读 {} 失败：{err}", path.display())),
    }
}

/// 原子写入：先写临时文件并 fsync，再 rename 覆盖目标。
#[tauri::command]
fn write_data(app: tauri::AppHandle, name: String, contents: String) -> Result<(), String> {
    let dir = data_dir(&app)?;
    let name = safe_name(&name)?;
    let path = dir.join(name);
    let tmp = dir.join(format!(".{name}.tmp"));
    let mut file =
        fs::File::create(&tmp).map_err(|err| format!("写 {} 失败：{err}", tmp.display()))?;
    file.write_all(contents.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|err| format!("写 {} 失败：{err}", tmp.display()))?;
    drop(file);
    fs::rename(&tmp, &path).map_err(|err| format!("替换 {} 失败：{err}", path.display()))?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![read_data, write_data])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
