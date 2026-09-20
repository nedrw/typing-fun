//! 小鹤双拼键位表（参考资料，练习时显示）。
//!
//! 声母就是首字母，只有三个例外：zh -> v、ch -> i、sh -> u。
//! 韵母按本表映射到一个键；零声母音节按「首字母 + 韵母键」输入。
//! 这只是一张参考表，实际以使用者输入法里的小鹤方案为准。

/// 26 个字母键各自承载的韵母（v/i/u 承载的是 zh/ch/sh）。
pub const XIAOHE: [(char, &str); 26] = [
    ('q', "iu"),
    ('w', "ei"),
    ('e', "e"),
    ('r', "uan"),
    ('t', "ue"),
    ('y', "un"),
    ('u', "sh"),
    ('i', "ch"),
    ('o', "uo"),
    ('p', "ie"),
    ('a', "a"),
    ('s', "ong"),
    ('d', "ai"),
    ('f', "en"),
    ('g', "eng"),
    ('h', "ang"),
    ('j', "an"),
    ('k', "ing"),
    ('l', "iang"),
    ('z', "ou"),
    ('x', "ia"),
    ('c', "ao"),
    ('v', "zh"),
    ('b', "in"),
    ('n', "iao"),
    ('m', "ian"),
];

/// 三排键位，用于按物理键盘的样子渲染参考表。
pub const ROWS: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];

/// 某个键承载的韵母。
pub fn yunmu(key: char) -> Option<&'static str> {
    XIAOHE.iter().find(|(k, _)| *k == key).map(|(_, y)| *y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_letters_are_covered_exactly_once() {
        let mut keys: Vec<char> = XIAOHE.iter().map(|(k, _)| *k).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), 26, "26 个字母都应出现在表里");
        let mut rows: Vec<char> = ROWS.iter().flat_map(|r| r.chars()).collect();
        rows.sort_unstable();
        assert_eq!(rows, keys, "键盘三排应正好覆盖表里的键");
    }

    #[test]
    fn known_entries_match_the_scheme() {
        assert_eq!(yunmu('q'), Some("iu"));
        assert_eq!(yunmu('w'), Some("ei"));
        assert_eq!(yunmu('r'), Some("uan"));
        assert_eq!(yunmu('t'), Some("ue"));
        assert_eq!(yunmu('y'), Some("un"));
        assert_eq!(yunmu('o'), Some("uo"));
        assert_eq!(yunmu('p'), Some("ie"));
        assert_eq!(yunmu('s'), Some("ong"));
        assert_eq!(yunmu('f'), Some("en"));
        assert_eq!(yunmu('g'), Some("eng"));
        assert_eq!(yunmu('h'), Some("ang"));
        assert_eq!(yunmu('j'), Some("an"));
        assert_eq!(yunmu('k'), Some("ing"));
        assert_eq!(yunmu('l'), Some("iang"));
        assert_eq!(yunmu('z'), Some("ou"));
        assert_eq!(yunmu('x'), Some("ia"));
        assert_eq!(yunmu('c'), Some("ao"));
        assert_eq!(yunmu('b'), Some("in"));
        assert_eq!(yunmu('n'), Some("iao"));
        assert_eq!(yunmu('m'), Some("ian"));
        // zh/ch/sh 这三个声母不在首字母位置
        assert_eq!(yunmu('v'), Some("zh"));
        assert_eq!(yunmu('i'), Some("ch"));
        assert_eq!(yunmu('u'), Some("sh"));
    }
}
