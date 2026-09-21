//! 美式键盘布局与指法映射（纯逻辑，不依赖 UI）。
//!
//! TT 的核心教学手段之一就是把「该用哪根手指」直接标出来，这里提供这份映射。

/// 参与打字的九根手指。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Finger {
    LPinky,
    LRing,
    LMiddle,
    LIndex,
    Thumb,
    RIndex,
    RMiddle,
    RRing,
    RPinky,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hand {
    Left,
    Right,
}

pub fn hand(f: Finger) -> Hand {
    match f {
        Finger::LPinky | Finger::LRing | Finger::LMiddle | Finger::LIndex => Hand::Left,
        _ => Hand::Right,
    }
}

/// 四排键位，索引 0 为数字排。
pub const ROWS: [&str; 4] = [
    "`1234567890-=",
    "qwertyuiop[]\\",
    "asdfghjkl;'",
    "zxcvbnm,./",
];

/// 归一化到键盘上的基础键：'A' -> 'a'，'?' -> '/'，' ' -> ' '。
pub fn base_key(ch: char) -> Option<char> {
    let base = match ch {
        'A'..='Z' => ch.to_ascii_lowercase(),
        '~' => '`',
        '!' => '1',
        '@' => '2',
        '#' => '3',
        '$' => '4',
        '%' => '5',
        '^' => '6',
        '&' => '7',
        '*' => '8',
        '(' => '9',
        ')' => '0',
        '_' => '-',
        '+' => '=',
        '{' => '[',
        '}' => ']',
        '|' => '\\',
        ':' => ';',
        '"' => '\'',
        '<' => ',',
        '>' => '.',
        '?' => '/',
        c => c,
    };
    if base == ' ' || ROWS.iter().any(|row| row.contains(base)) {
        Some(base)
    } else {
        None
    }
}

/// 该字符需要 Shift（大写字母或上档符号）。
pub fn needs_shift(ch: char) -> bool {
    match ch {
        'A'..='Z' => true,
        c => base_key(c).is_some_and(|b| b.is_ascii() && c != b),
    }
}

/// 字符对应的手指。
pub fn finger_of(ch: char) -> Option<Finger> {
    use Finger::*;
    Some(match base_key(ch)? {
        ' ' => Thumb,
        '`' | '1' | 'q' | 'a' | 'z' => LPinky,
        '2' | 'w' | 's' | 'x' => LRing,
        '3' | 'e' | 'd' | 'c' => LMiddle,
        '4' | '5' | 'r' | 't' | 'f' | 'g' | 'v' | 'b' => LIndex,
        '6' | '7' | 'y' | 'u' | 'h' | 'j' | 'n' | 'm' => RIndex,
        '8' | 'i' | 'k' | ',' => RMiddle,
        '9' | 'o' | 'l' | '.' => RRing,
        '0' | '-' | '=' | 'p' | '[' | ']' | '\\' | ';' | '\'' | '/' => RPinky,
        _ => return None,
    })
}

pub fn finger_name(f: Finger) -> &'static str {
    use Finger::*;
    match f {
        LPinky => "左手小指",
        LRing => "左手无名指",
        LMiddle => "左手中指",
        LIndex => "左手食指",
        Thumb => "拇指",
        RIndex => "右手食指",
        RMiddle => "右手中指",
        RRing => "右手无名指",
        RPinky => "右手小指",
    }
}

/// 手指对应的 CSS 颜色类。
pub fn finger_class(f: Finger) -> &'static str {
    use Finger::*;
    match f {
        LPinky => "f-lp",
        LRing => "f-lr",
        LMiddle => "f-lm",
        LIndex => "f-li",
        Thumb => "f-th",
        RIndex => "f-ri",
        RMiddle => "f-rm",
        RRing => "f-rr",
        RPinky => "f-rp",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uppercase_maps_to_letter_key() {
        assert_eq!(base_key('A'), Some('a'));
        assert!(needs_shift('A'));
        assert!(!needs_shift('a'));
        assert_eq!(finger_of('A'), finger_of('a'));
    }

    #[test]
    fn shifted_symbols_map_to_base_keys() {
        assert_eq!(base_key('?'), Some('/'));
        assert_eq!(base_key(':'), Some(';'));
        assert_eq!(base_key('_'), Some('-'));
        assert_eq!(base_key('"'), Some('\''));
        assert!(needs_shift('?'));
        assert!(!needs_shift('/'));
    }

    #[test]
    fn home_row_fingers_are_standard() {
        assert_eq!(finger_of('a'), Some(Finger::LPinky));
        assert_eq!(finger_of('s'), Some(Finger::LRing));
        assert_eq!(finger_of('d'), Some(Finger::LMiddle));
        assert_eq!(finger_of('f'), Some(Finger::LIndex));
        assert_eq!(finger_of('j'), Some(Finger::RIndex));
        assert_eq!(finger_of('k'), Some(Finger::RMiddle));
        assert_eq!(finger_of('l'), Some(Finger::RRing));
        assert_eq!(finger_of(';'), Some(Finger::RPinky));
        assert_eq!(finger_of(' '), Some(Finger::Thumb));
    }

    #[test]
    fn hand_split_matches_finger() {
        assert_eq!(hand(Finger::LIndex), Hand::Left);
        assert_eq!(hand(Finger::Thumb), Hand::Right);
        assert_eq!(hand(Finger::RPinky), Hand::Right);
    }

    #[test]
    fn every_lesson_char_maps_to_a_key_and_finger() {
        for ch in "`1234567890-=qwertyuiop[]\\asdfghjkl;'zxcvbnm,./ ".chars() {
            assert_eq!(base_key(ch), Some(ch), "{ch} 无法映射到键位");
            assert!(finger_of(ch).is_some(), "{ch} 没有对应手指");
        }
        assert_eq!(base_key('中'), None);
    }
}
