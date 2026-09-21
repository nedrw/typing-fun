//! 成绩记录模型与环形容量（落盘由 `crate::store` 负责，文件 `records.ron`）。

use serde::{Deserialize, Serialize};

/// 用户成绩文件名（相对 app data dir）。
pub const FILE: &str = "records.ron";

/// 最多保留的成绩条数；超出丢最旧的，避免文件无限增长。
pub const CAP: usize = 500;

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
    /// 错误分布（英文问题键 / 中文错字），按首次打错累计，倒序截 Top N
    #[serde(default)]
    pub errors_by_key: Vec<(char, u32)>,
}

/// 追加一条记录并按容量裁剪，返回新列表。
pub fn push(all: &[Record], record: Record) -> Vec<Record> {
    let mut next: Vec<Record> = all.to_vec();
    next.push(record);
    trim(next)
}

/// 裁剪到容量上限（丢最旧的）。
pub fn trim(mut all: Vec<Record>) -> Vec<Record> {
    if all.len() > CAP {
        all.drain(..all.len() - CAP);
    }
    all
}

/// 某课程/项目的最快记录。
pub fn best<'a>(all: &'a [Record], lesson_id: &str) -> Option<&'a Record> {
    all.iter()
        .filter(|r| r.lesson_id == lesson_id)
        .max_by(|a, b| a.cpm.total_cmp(&b.cpm))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, cpm: f64) -> Record {
        Record {
            lesson_id: id.to_string(),
            title: id.to_string(),
            at_ms: 0.0,
            cpm,
            accuracy: 100.0,
            errors: 0,
            chars: 0,
            secs: 1.0,
            errors_by_key: Vec::new(),
        }
    }

    #[test]
    fn push_keeps_newest_within_capacity() {
        let mut all: Vec<Record> = (0..CAP).map(|i| record("x", i as f64)).collect();
        all = push(&all, record("x", 9999.0));
        assert_eq!(all.len(), CAP);
        assert_eq!(all.last().unwrap().cpm, 9999.0);
        assert_eq!(all.first().unwrap().cpm, 1.0, "最旧的一条被丢掉");
    }

    #[test]
    fn trim_handles_oversized_files() {
        let all: Vec<Record> = (0..CAP + 100).map(|i| record("x", i as f64)).collect();
        let trimmed = trim(all);
        assert_eq!(trimmed.len(), CAP);
        assert_eq!(trimmed.last().unwrap().cpm, (CAP + 99) as f64);
    }

    #[test]
    fn best_picks_highest_cpm_per_item() {
        let all = vec![record("a", 100.0), record("b", 900.0), record("a", 300.0)];
        assert_eq!(best(&all, "a").unwrap().cpm, 300.0);
        assert_eq!(best(&all, "b").unwrap().cpm, 900.0);
        assert!(best(&all, "c").is_none());
    }

    #[test]
    fn errors_by_key_is_optional_for_old_records() {
        let old = r#"(lesson_id:"home-fj",at_ms:0.0,cpm:120.0,accuracy:98.0,errors:2,chars:40,secs:20.0)"#;
        let parsed: Record = ron::from_str(old).unwrap();
        assert!(parsed.errors_by_key.is_empty());
        assert_eq!(parsed.title, "");
    }
}
