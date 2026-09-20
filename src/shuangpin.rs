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
}
