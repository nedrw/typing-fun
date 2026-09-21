//! 界面设置（settings.ron）：目前只记住上次用的语言页。

use serde::{Deserialize, Serialize};

use crate::lessons::Lang;

/// 设置文件名（相对 app data dir）。
pub const FILE: &str = "settings.ron";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub lang: Lang,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_lang_falls_back_to_english() {
        let parsed: Settings = ron::from_str("()").unwrap();
        assert_eq!(parsed.lang, Lang::En);
    }

    #[test]
    fn lang_round_trips_as_rust_enum() {
        let text = ron::ser::to_string(&Settings { lang: Lang::Zh }).unwrap();
        assert_eq!(text, "(lang:Zh)");
        assert_eq!(ron::from_str::<Settings>(&text).unwrap().lang, Lang::Zh);
    }
}
