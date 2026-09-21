//! 构建脚本：读 `assets/materials/manifest.toml` 与 `assets/pinyin/pinyin.txt`，
//! 生成随包素材表与汉字拼音表。
//!
//! 生成的是 `pub const BUNDLED: &[(&str, &str, Lang, &str)]`（正文用 `include_str!`
//! 指向 txt 的绝对路径）与排序好的 `pub static HANZI: &[(char, &str)]`，
//! 所以运行期不读文件、不发请求。
//!
//! 清单或正文有问题一律在**构建期**失败，而不是等运行起来发现列表是空的：
//! 文件读不到、语言非法、id/显示名重复、正文为空、英文素材含非 ASCII 字符。
//! 拼音表只收常用汉字区（U+4E00–U+9FFF）与「〇」，并在这里去调、ü 归一成 v。

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize)]
struct Manifest {
    #[serde(default)]
    material: Vec<Entry>,
}

#[derive(serde::Deserialize)]
struct Entry {
    id: String,
    name: String,
    lang: String,
    file: String,
}

fn main() {
    let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let materials_dir = crate_dir.join("assets").join("materials");
    let manifest_path = materials_dir.join("manifest.toml");

    println!("cargo:rerun-if-changed={}", manifest_path.display());
    println!("cargo:rerun-if-changed={}", materials_dir.display());

    let raw = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|err| panic!("读不到素材清单 {}：{err}", manifest_path.display()));
    let manifest: Manifest = toml::from_str(&raw)
        .unwrap_or_else(|err| panic!("解析素材清单 {} 失败：{err}", manifest_path.display()));
    assert!(
        !manifest.material.is_empty(),
        "{} 里一段素材都没有",
        manifest_path.display()
    );

    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    let mut listed = HashSet::new();
    let mut table = String::new();

    for entry in &manifest.material {
        let lang = match entry.lang.as_str() {
            "en" => "crate::lessons::Lang::En",
            "zh" => "crate::lessons::Lang::Zh",
            other => panic!("素材 {} 的语言 {other:?} 非法（只能是 en 或 zh）", entry.id),
        };
        assert!(ids.insert(entry.id.clone()), "素材 id 重复：{}", entry.id);
        assert!(
            names.insert(entry.name.clone()),
            "素材显示名重复：{}",
            entry.name
        );

        let path = materials_dir.join(&entry.file);
        assert!(
            listed.insert(entry.file.clone()),
            "素材文件被清单引用了两次：{}",
            entry.file
        );
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("读不到素材正文 {}：{err}", path.display()));
        check_typeable(&entry.id, &entry.name, &entry.lang, &text);

        table.push_str(&format!(
            "    ({:?}, {:?}, {}, include_str!({:?})),\n",
            entry.id,
            entry.name,
            lang,
            path.to_string_lossy()
        ));
    }

    warn_about_unlisted_files(&materials_dir, &listed);
    build_pinyin_table(&crate_dir);

    let generated = format!(
        "// 由 build.rs 依据 assets/materials/manifest.toml 生成，请勿手改。\n\
         pub const BUNDLED: &[(&str, &str, crate::lessons::Lang, &str)] = &[\n{table}];\n"
    );
    let out_path =
        PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("bundled_materials.rs");
    fs::write(&out_path, generated)
        .unwrap_or_else(|err| panic!("写 {} 失败：{err}", out_path.display()));
}

/// 素材必须「打得出来」：英文素材只能是可打印 ASCII；
/// 中文素材允许夹英文（中文输入法下可用英文模式或回车直接提交字母），仅要求非空。
fn check_typeable(id: &str, name: &str, lang: &str, text: &str) {
    assert!(!text.trim().is_empty(), "素材 {id}（{name}）正文是空的");
    if lang == "en" {
        if let Some(bad) = text.chars().find(|c| !c.is_ascii()) {
            panic!("英文素材 {id}（{name}）含非 ASCII 字符 {bad:?}，键盘上打不出来");
        }
    }
}

/// 目录里有 txt 没被清单引用时给个提醒（不影响构建）。
fn warn_about_unlisted_files(dir: &Path, listed: &HashSet<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".txt") && !listed.contains(&name) {
            println!("cargo:warning=素材 {name} 没有被 manifest.toml 引用，不会出现在菜单里");
        }
    }
}

// ---------- 拼音表 ----------

/// 汉字 → 拼音（无调、多音逗号分隔）的源文件（pinyin-data，MIT）。
fn build_pinyin_table(crate_dir: &Path) {
    let path = crate_dir.join("assets").join("pinyin").join("pinyin.txt");
    println!("cargo:rerun-if-changed={}", path.display());
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("读不到拼音表 {}：{err}", path.display()));

    let mut table: Vec<(char, String)> = Vec::new();
    for (lineno, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let number = lineno + 1;
        let Some((code, rest)) = line.split_once(':') else {
            panic!("拼音表第 {number} 行缺少冒号：{line}");
        };
        let hex = code.trim().trim_start_matches("U+");
        let codepoint = u32::from_str_radix(hex, 16)
            .unwrap_or_else(|err| panic!("拼音表第 {number} 行的码位非法：{code}（{err}）"));
        let ch = char::from_u32(codepoint)
            .unwrap_or_else(|| panic!("拼音表第 {number} 行的码位不是字符：{code}"));
        // 只保留常用汉字区与「〇」；扩展区等生僻字不进二进制
        if !(('\u{4e00}'..='\u{9fff}').contains(&ch) || ch == '\u{3007}') {
            continue;
        }
        let readings = rest.split('#').next().unwrap_or("");
        let mut list: Vec<String> = Vec::new();
        for reading in readings.split(',') {
            let plain = strip_tones(reading);
            if !plain.is_empty() && !list.contains(&plain) {
                list.push(plain);
            }
        }
        if list.is_empty() {
            panic!("拼音表第 {number} 行的读音无法归一化：{line}");
        }
        table.push((ch, list.join(",")));
    }

    table.sort_by_key(|(ch, _)| *ch);
    assert!(
        table.len() > 20_000,
        "拼音表只解析出 {} 个字，检查 {}",
        table.len(),
        path.display()
    );

    let mut generated = String::from(
        "// 由 build.rs 依据 assets/pinyin/pinyin.txt 生成，请勿手改。\n\
         /// 汉字 -> 拼音（无调，多音逗号分隔，按常用度排序），按码位升序。\n\
         pub static HANZI: &[(char, &str)] = &[\n",
    );
    for (ch, readings) in &table {
        generated.push_str(&format!("    ({ch:?}, {readings:?}),\n"));
    }
    generated.push_str("];\n");

    let out_path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("pinyin_table.rs");
    fs::write(&out_path, generated)
        .unwrap_or_else(|err| panic!("写 {} 失败：{err}", out_path.display()));
}

/// 去掉声调：ā→a、ǖ→v，其余非 ASCII 音标兼容字母映射到基字母。
fn strip_tones(reading: &str) -> String {
    reading
        .chars()
        .filter_map(|c| match c {
            'a'..='z' | 'A'..='Z' => Some(c.to_ascii_lowercase()),
            'ā' | 'á' | 'ǎ' | 'à' => Some('a'),
            'ē' | 'é' | 'ě' | 'è' | 'ê' => Some('e'),
            'ī' | 'í' | 'ǐ' | 'ì' => Some('i'),
            'ō' | 'ó' | 'ǒ' | 'ò' => Some('o'),
            'ū' | 'ú' | 'ǔ' | 'ù' => Some('u'),
            'ü' | 'ǖ' | 'ǘ' | 'ǚ' | 'ǜ' => Some('v'),
            'ń' | 'ň' | 'ǹ' => Some('n'),
            'ḿ' => Some('m'),
            _ => None,
        })
        .collect()
}
