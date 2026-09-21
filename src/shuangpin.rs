//! 小鹤双拼键位表（参考资料，练习时显示）。
//!
//! - 声母：除 zh → V、ch → I、sh → U 之外，其余声母就是对应字母键
//! - 韵母：一个韵母占一个键（下表），T/O/S/K/L/X/V 等键承载两个韵母
//! - 零声母音节（安 an、爱 ai 等）通常把首字母当声母处理
//! - 「助记词」一列是官方键位图的谐音口诀，便于记忆
//!
//! 这只是一张参考表，实际以使用者输入法里的小鹤方案为准。

pub struct Key {
    pub key: char,
    /// 该键承载的韵母，多个用「、」分隔
    pub yunmu: &'static str,
    /// 该键兼作的声母（只有 zh / ch / sh 例外，其余为空）
    pub initial: &'static str,
    /// 助记词，可能为空（e / a / o 三键没有）
    pub mnemonic: &'static str,
}

pub const XIAOHE: [Key; 26] = [
    Key {
        key: 'q',
        yunmu: "iu",
        initial: "",
        mnemonic: "秋",
    },
    Key {
        key: 'w',
        yunmu: "ei",
        initial: "",
        mnemonic: "闹",
    },
    Key {
        key: 'e',
        yunmu: "e",
        initial: "",
        mnemonic: "",
    },
    Key {
        key: 'r',
        yunmu: "uan",
        initial: "",
        mnemonic: "软",
    },
    Key {
        key: 't',
        yunmu: "ue、üe",
        initial: "",
        mnemonic: "月",
    },
    Key {
        key: 'y',
        yunmu: "un",
        initial: "",
        mnemonic: "云",
    },
    Key {
        key: 'u',
        yunmu: "u",
        initial: "sh",
        mnemonic: "梳",
    },
    Key {
        key: 'i',
        yunmu: "i",
        initial: "ch",
        mnemonic: "翅",
    },
    Key {
        key: 'o',
        yunmu: "o、uo",
        initial: "",
        mnemonic: "",
    },
    Key {
        key: 'p',
        yunmu: "ie",
        initial: "",
        mnemonic: "撇",
    },
    Key {
        key: 'a',
        yunmu: "a",
        initial: "",
        mnemonic: "",
    },
    Key {
        key: 's',
        yunmu: "ong、iong",
        initial: "",
        mnemonic: "怂恿",
    },
    Key {
        key: 'd',
        yunmu: "ai",
        initial: "",
        mnemonic: "带",
    },
    Key {
        key: 'f',
        yunmu: "en",
        initial: "",
        mnemonic: "粉",
    },
    Key {
        key: 'g',
        yunmu: "eng",
        initial: "",
        mnemonic: "更",
    },
    Key {
        key: 'h',
        yunmu: "ang",
        initial: "",
        mnemonic: "航",
    },
    Key {
        key: 'j',
        yunmu: "an",
        initial: "",
        mnemonic: "安",
    },
    Key {
        key: 'k',
        yunmu: "uai、ing",
        initial: "",
        mnemonic: "快迎",
    },
    Key {
        key: 'l',
        yunmu: "iang、uang",
        initial: "",
        mnemonic: "两王",
    },
    Key {
        key: 'z',
        yunmu: "ou",
        initial: "",
        mnemonic: "揍",
    },
    Key {
        key: 'x',
        yunmu: "ia、ua",
        initial: "",
        mnemonic: "夏娃",
    },
    Key {
        key: 'c',
        yunmu: "ao",
        initial: "",
        mnemonic: "草",
    },
    Key {
        key: 'v',
        yunmu: "ui、ü",
        initial: "zh",
        mnemonic: "追鱼",
    },
    Key {
        key: 'b',
        yunmu: "in",
        initial: "",
        mnemonic: "滨",
    },
    Key {
        key: 'n',
        yunmu: "iao",
        initial: "",
        mnemonic: "鸟",
    },
    Key {
        key: 'm',
        yunmu: "ian",
        initial: "",
        mnemonic: "眠",
    },
];

/// 三排键位，用于按物理键盘的样子渲染参考表。
pub const ROWS: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];

pub fn key_at(key: char) -> Option<&'static Key> {
    XIAOHE.iter().find(|entry| entry.key == key)
}

/// 声母键：zh→v、ch→i、sh→u，其余声母就是字母本身。
fn initial_key(initial: &str) -> Option<char> {
    match initial {
        "zh" => Some('v'),
        "ch" => Some('i'),
        "sh" => Some('u'),
        _ => {
            let mut chars = initial.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if "bpmfdtnlgkhjqxrzcsyw".contains(c) => Some(c),
                _ => None,
            }
        }
    }
}

/// 韵母键（与 `XIAOHE` 表一致，ü 记成 v）。
fn final_key(final_sound: &str) -> Option<char> {
    Some(match final_sound {
        "a" => 'a',
        "o" => 'o',
        "e" => 'e',
        "i" => 'i',
        "u" => 'u',
        "v" => 'v',
        "ai" => 'd',
        "ei" => 'w',
        "ao" => 'c',
        "ou" => 'z',
        "an" => 'j',
        "en" => 'f',
        "ang" => 'h',
        "eng" => 'g',
        "ong" => 's',
        "er" => 'r',
        "ia" => 'x',
        "ie" => 'p',
        "iao" => 'n',
        "iu" => 'q',
        "ian" => 'm',
        "in" => 'b',
        "iang" => 'l',
        "ing" => 'k',
        "iong" => 's',
        "ua" => 'x',
        "uo" => 'o',
        "uai" => 'k',
        "ui" => 'v',
        "uan" => 'r',
        "un" => 'y',
        "uang" => 'l',
        "ueng" => 'g',
        "ue" => 't',
        "ve" => 't',
        "van" => 'r',
        "vn" => 'y',
        _ => return None,
    })
}

/// 拼音（无调、ü 写成 v）→ 小鹤双拼的两键编码。
///
/// 零声母音节把首字母当声母键：安 an → `a j`、爱 ai → `a d`、儿 er → `e r`。
/// 音节表覆盖不到的读音（呼 hm、唔 ng 等）返回 `None`，调用方可以选择丢弃。
pub fn encode(pinyin: &str) -> Option<[char; 2]> {
    let pinyin = pinyin.trim();
    let (initial, final_sound) = split_syllable(pinyin);
    if final_sound.is_empty() {
        return None;
    }
    let first = if initial.is_empty() {
        let first = final_sound.chars().next()?;
        if !"aoe".contains(first) {
            return None;
        }
        first
    } else {
        initial_key(initial)?
    };
    Some([first, final_key(final_sound)?])
}

/// 拆声母 / 韵母；零声母时声母为空串、韵母是整个音节。
fn split_syllable(pinyin: &str) -> (&str, &str) {
    for prefix in ["zh", "ch", "sh"] {
        if let Some(rest) = pinyin.strip_prefix(prefix) {
            return (prefix, rest);
        }
    }
    if let Some(first) = pinyin.chars().next() {
        if "bpmfdtnlgkhjqxrzcsyw".contains(first) {
            return pinyin.split_at(first.len_utf8());
        }
    }
    ("", pinyin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_letters_are_covered_exactly_once() {
        let mut keys: Vec<char> = XIAOHE.iter().map(|entry| entry.key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), 26, "26 个字母都应出现在表里");
        let mut rows: Vec<char> = ROWS.iter().flat_map(|r| r.chars()).collect();
        rows.sort_unstable();
        assert_eq!(rows, keys, "键盘三排应正好覆盖表里的键");
    }

    #[test]
    fn known_entries_match_the_scheme() {
        let yunmu = |key: char| key_at(key).unwrap().yunmu;
        assert_eq!(yunmu('q'), "iu");
        assert_eq!(yunmu('w'), "ei");
        assert_eq!(yunmu('e'), "e");
        assert_eq!(yunmu('r'), "uan");
        assert_eq!(yunmu('t'), "ue、üe");
        assert_eq!(yunmu('y'), "un");
        assert_eq!(yunmu('u'), "u");
        assert_eq!(yunmu('i'), "i");
        assert_eq!(yunmu('o'), "o、uo");
        assert_eq!(yunmu('p'), "ie");
        assert_eq!(yunmu('a'), "a");
        assert_eq!(yunmu('s'), "ong、iong");
        assert_eq!(yunmu('d'), "ai");
        assert_eq!(yunmu('f'), "en");
        assert_eq!(yunmu('g'), "eng");
        assert_eq!(yunmu('h'), "ang");
        assert_eq!(yunmu('j'), "an");
        assert_eq!(yunmu('k'), "uai、ing");
        assert_eq!(yunmu('l'), "iang、uang");
        assert_eq!(yunmu('z'), "ou");
        assert_eq!(yunmu('x'), "ia、ua");
        assert_eq!(yunmu('c'), "ao");
        assert_eq!(yunmu('v'), "ui、ü");
        assert_eq!(yunmu('b'), "in");
        assert_eq!(yunmu('n'), "iao");
        assert_eq!(yunmu('m'), "ian");
    }

    #[test]
    fn only_zh_ch_sh_take_extra_keys() {
        for entry in XIAOHE.iter() {
            let expected = match entry.key {
                'v' => "zh",
                'i' => "ch",
                'u' => "sh",
                _ => "",
            };
            assert_eq!(
                entry.initial, expected,
                "键 {} 的兼作声母应为「{}」",
                entry.key, expected
            );
        }
    }

    #[test]
    fn every_yunmu_has_exactly_one_key() {
        let mut seen: Vec<&str> = Vec::new();
        for entry in XIAOHE.iter() {
            for yunmu in entry.yunmu.split('、') {
                assert!(
                    !seen.contains(&yunmu),
                    "韵母 {yunmu} 出现在多个键上：{seen:?}"
                );
                seen.push(yunmu);
            }
        }
        assert_eq!(
            seen.len(),
            33,
            "小鹤表应覆盖 33 个韵母，实际 {}",
            seen.len()
        );
    }

    #[test]
    fn every_key_has_yunmu_and_short_mnemonic() {
        for entry in XIAOHE.iter() {
            assert!(!entry.yunmu.is_empty(), "键 {} 没有韵母", entry.key);
            assert!(
                entry.mnemonic.chars().count() <= 3,
                "键 {} 的助记词太长：{}",
                entry.key,
                entry.mnemonic
            );
        }
    }

    #[test]
    fn known_pinyin_encodes_to_xiaohe() {
        let encode = |pinyin| super::encode(pinyin);
        assert_eq!(encode("zhong"), Some(['v', 's']));
        assert_eq!(encode("de"), Some(['d', 'e']));
        assert_eq!(encode("shuang"), Some(['u', 'l']));
        assert_eq!(encode("ying"), Some(['y', 'k']));
        assert_eq!(encode("wei"), Some(['w', 'w']));
        assert_eq!(encode("ju"), Some(['j', 'u']));
        assert_eq!(encode("jun"), Some(['j', 'y']));
        assert_eq!(encode("jue"), Some(['j', 't']));
        assert_eq!(encode("yuan"), Some(['y', 'r']));
        assert_eq!(encode("nv"), Some(['n', 'v']));
        assert_eq!(encode("lve"), Some(['l', 't']));
    }

    #[test]
    fn zero_initial_syllables_prefix_the_letter() {
        assert_eq!(super::encode("an"), Some(['a', 'j']));
        assert_eq!(super::encode("ai"), Some(['a', 'd']));
        assert_eq!(super::encode("ang"), Some(['a', 'h']));
        assert_eq!(super::encode("er"), Some(['e', 'r']));
        assert_eq!(super::encode("ou"), Some(['o', 'z']));
        assert_eq!(super::encode("e"), Some(['e', 'e']));
    }

    #[test]
    fn unknown_syllables_return_none() {
        assert_eq!(super::encode(""), None);
        assert_eq!(super::encode("ng"), None, "呼读音节没有双拼码");
        assert_eq!(super::encode("m"), None);
        assert_eq!(super::encode("zzz"), None);
    }

    /// 拼音表里除呼读音节（n / ng / m / hm / hng）外都必须能编码；
    /// 表更新后如果有新韵母漏掉，这个测试会失败。
    #[test]
    fn pinyin_table_readings_are_encodable() {
        use crate::pinyin::HANZI;
        for (ch, readings) in HANZI {
            for reading in readings.split(',') {
                if super::encode(reading).is_none() {
                    assert!(
                        matches!(reading, "n" | "ng" | "m" | "hm" | "hng"),
                        "{ch} 的读音 {reading} 没有双拼编码"
                    );
                }
            }
        }
    }
}
