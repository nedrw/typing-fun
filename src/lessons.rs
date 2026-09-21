//! 课程表与练习文本生成（纯逻辑，不依赖 UI）。
//!
//! 英文课程沿 TT 的路线推进：基准键 -> 上排 -> 下排 -> 数字 -> 单词 -> 文章，
//! 后面的键位课都会带上前面学过的键，形成复习。
//! 中文课程走输入法：常用字 -> 常用词 -> 短文，文本里不含 ASCII
//! （输入法下字母是拼音，打不出字母；空格是选字键，所以也不依赖空格）。

use serde::{Deserialize, Serialize};

use crate::rng::Rng;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Lang {
    #[default]
    En,
    Zh,
}

impl Lang {
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "英文",
            Lang::Zh => "中文",
        }
    }
}

/// 课程分组（菜单里的级别）。
pub struct LessonGroup {
    pub id: &'static str,
    pub title: &'static str,
    pub lang: Lang,
}

/// 练习文本的生成方式。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LessonKind {
    /// 键位练习：从 keys 里随机取字符，4 个一组（英文用空格分组）
    Drill { keys: &'static str },
    /// 字表练习：从池子里随机取单字，中文不加分隔
    Chars { pool: &'static str },
    /// 词语练习：从词库随机抽词，`sep` 为词间分隔（中文为空字符串）
    Words {
        pool: &'static [&'static str],
        sep: &'static str,
    },
    /// 文章练习：固定文本
    Text { text: &'static str },
}

pub struct Lesson {
    pub id: &'static str,
    pub title: &'static str,
    pub desc: &'static str,
    pub lang: Lang,
    /// 所属分组，对应 `GROUPS` 里的 id
    pub group: &'static str,
    pub kind: LessonKind,
    /// 生成文本的目标字符数（Text 类忽略）
    pub len: usize,
}

impl Lesson {
    /// 生成一份练习文本。每次开始练习换一个 seed，避免每次都是同一串。
    pub fn generate(&self, seed: u64) -> String {
        let mut rng = Rng::new(seed);
        match self.kind {
            LessonKind::Drill { keys } => {
                let chars: Vec<char> = keys.chars().filter(|c| !c.is_whitespace()).collect();
                let mut out = String::new();
                let mut drawn = 0;
                while drawn < self.len {
                    for _ in 0..4 {
                        out.push(chars[rng.below(chars.len())]);
                        drawn += 1;
                    }
                    out.push(' ');
                }
                out.trim_end().to_string()
            }
            LessonKind::Chars { pool } => {
                let chars: Vec<char> = pool.chars().filter(|c| !c.is_whitespace()).collect();
                (0..self.len)
                    .map(|_| chars[rng.below(chars.len())])
                    .collect()
            }
            LessonKind::Words { pool, sep } => {
                let mut out = String::new();
                let mut drawn = 0;
                while drawn < self.len {
                    if drawn > 0 {
                        out.push_str(sep);
                        drawn += sep.chars().count();
                    }
                    let word = pool[rng.below(pool.len())];
                    out.push_str(word);
                    drawn += word.chars().count();
                }
                out
            }
            LessonKind::Text { text } => text.to_string(),
        }
    }
}

const HOME: &str = "asdfjkl;";

static WORDS: &[&str] = &[
    "the", "and", "for", "you", "that", "with", "have", "this", "from", "they", "know", "want",
    "been", "good", "much", "some", "time", "very", "when", "come", "here", "just", "like", "long",
    "make", "many", "more", "only", "over", "such", "take", "than", "them", "well", "were", "what",
    "work", "your", "about", "would", "there", "their", "which", "could", "other", "after",
    "first", "never", "these", "think", "where", "being", "every", "great", "might", "shall",
    "still", "those", "under", "while", "world", "house", "large", "small", "light", "night",
    "right", "point", "place", "again", "found", "sound", "young", "three", "water", "heart",
    "music", "study", "learn", "paper", "plant", "green", "quick", "brown", "jumps", "lazy",
    "finger", "keyboard", "screen", "letter", "number", "second", "minute", "always", "before",
    "better", "between", "because", "through", "another", "country", "morning", "perfect",
    "practice",
];

const TEXT_SHIFT: &str = "The Quick Brown Fox Jumps Over The Lazy Dog. Pack My Box With Five \
Dozen Liquor Jugs. How Vexingly Quick Daft Zebras Jump! Typing Well Needs Capital Letters Too.";

const TEXT_PASSAGE: &str = "Touch typing means typing without looking at the keyboard. Your \
fingers learn where every key lives, and your eyes stay on the screen. Speed comes from accuracy, \
so keep your hands relaxed, your wrists light, and your rhythm steady. Practice a little every \
day and the keys will feel like they were always yours.";

/// 常用汉字（按频次排的常用字表），字表练习的池子。
const HANZI_1: &str = "的一是了我不人在他有这个上们来到时大地为子中你说生国年着就那和要她出也得里后自以会家可下而过天去能对小多然于心学么之都好看起发当没成只如事把还用第样道想作种开美总从无情己面最女但现前些所同日手又行意动方期它头经长儿回位分爱老因很给名法间斯知世什两次使身者被高已亲其进此话常与活正感";

/// 打字场景的常用字，字表练习的池子。
const HANZI_2: &str = "练习打字速度准确键盘手指位置屏幕眼睛休息坚持每天进步成绩提高训练方法字母符号标点数字文章段落句子词语汉字拼音输入选择候选空格回车删除修改正确错误重新开始结束时间分钟今天明天早上晚上朋友家人学习工作生活城市天气音乐电影旅行照片手机电脑网络世界中国北京上海广州深圳";

/// 中文常用词。
static CN_WORDS: &[&str] = &[
    "学习", "工作", "生活", "时间", "朋友", "中国", "世界", "电脑", "键盘", "练习", "速度", "准确",
    "认真", "每天", "进步", "老师", "学生", "图书", "房间", "城市", "天气", "早上", "晚上", "音乐",
    "电影", "旅行", "照片", "手机", "网络", "文章", "汉字", "拼音", "输入", "训练", "目标", "方法",
    "结果", "开始", "结束", "休息", "坚持", "改变", "习惯", "效率", "技能", "水平", "标准", "基础",
    "细节", "耐心", "信心", "收获", "成长", "记录", "课程",
];

const CN_PASSAGE_1: &str = "打字是一项技能。只要每天坚持练习，手指就会慢慢记住每一个键的位置。速度来自准确，先求准，再求快。";

const CN_PASSAGE_2: &str =
    "学习打字没有捷径，只有反复练习。看清屏幕上的字，不要看键盘。累了就休息一会儿，然后再继续。";

pub static GROUPS: &[LessonGroup] = &[
    LessonGroup {
        id: "en-home",
        title: "基础 · 基准键位",
        lang: Lang::En,
    },
    LessonGroup {
        id: "en-upper",
        title: "进阶 · 上排键位",
        lang: Lang::En,
    },
    LessonGroup {
        id: "en-lower",
        title: "进阶 · 下排与数字",
        lang: Lang::En,
    },
    LessonGroup {
        id: "en-text",
        title: "实战 · 单词与文章",
        lang: Lang::En,
    },
    LessonGroup {
        id: "zh-hanzi",
        title: "中文 · 常用字",
        lang: Lang::Zh,
    },
    LessonGroup {
        id: "zh-word",
        title: "中文 · 词语",
        lang: Lang::Zh,
    },
    LessonGroup {
        id: "zh-text",
        title: "中文 · 短文",
        lang: Lang::Zh,
    },
];

pub static LESSONS: &[Lesson] = &[
    Lesson {
        id: "home-fj",
        title: "第 1 课 · 基准键 F J",
        desc: "左手食指放在 F，右手食指放在 J，先熟悉这两个键",
        lang: Lang::En,
        group: "en-home",
        kind: LessonKind::Drill { keys: "fj" },
        len: 120,
    },
    Lesson {
        id: "home-dk",
        title: "第 2 课 · 基准键 D K",
        desc: "左手中指 D，右手中指 K",
        lang: Lang::En,
        group: "en-home",
        kind: LessonKind::Drill { keys: "asdfjkl;dk" },
        len: 120,
    },
    Lesson {
        id: "home-sl",
        title: "第 3 课 · 基准键 S L",
        desc: "左手无名指 S，右手无名指 L",
        lang: Lang::En,
        group: "en-home",
        kind: LessonKind::Drill { keys: "asdfjkl;sl" },
        len: 120,
    },
    Lesson {
        id: "home-a-semi",
        title: "第 4 课 · 基准键 A ;",
        desc: "左手小指 A，右手小指 ;（分号）",
        lang: Lang::En,
        group: "en-home",
        kind: LessonKind::Drill { keys: "asdfjkl;" },
        len: 120,
    },
    Lesson {
        id: "home-all",
        title: "第 5 课 · 基准键综合",
        desc: "八指归位，全程不看键盘",
        lang: Lang::En,
        group: "en-home",
        kind: LessonKind::Drill { keys: HOME },
        len: 160,
    },
    Lesson {
        id: "up-ei",
        title: "第 6 课 · 上排 E I",
        desc: "中指向上斜移一格",
        lang: Lang::En,
        group: "en-upper",
        kind: LessonKind::Drill { keys: "asdfjkl;ei" },
        len: 140,
    },
    Lesson {
        id: "up-ru",
        title: "第 7 课 · 上排 R U",
        desc: "食指向上斜移一格",
        lang: Lang::En,
        group: "en-upper",
        kind: LessonKind::Drill {
            keys: "asdfjkl;eiru",
        },
        len: 140,
    },
    Lesson {
        id: "up-ty",
        title: "第 8 课 · 上排 T Y",
        desc: "食指再向中间伸一格",
        lang: Lang::En,
        group: "en-upper",
        kind: LessonKind::Drill {
            keys: "asdfjkl;eiruty",
        },
        len: 140,
    },
    Lesson {
        id: "up-qwop",
        title: "第 9 课 · 上排 Q W O P",
        desc: "小指与无名指的上排键",
        lang: Lang::En,
        group: "en-upper",
        kind: LessonKind::Drill {
            keys: "asdfjkl;eirutyqwop",
        },
        len: 160,
    },
    Lesson {
        id: "down-all",
        title: "第 10 课 · 下排 Z X C V B N M , . /",
        desc: "下排先往下摸，再回到基准键",
        lang: Lang::En,
        group: "en-lower",
        kind: LessonKind::Drill {
            keys: "asdfjkl;eirutyqwopzxcvbnm,./",
        },
        len: 160,
    },
    Lesson {
        id: "digits",
        title: "第 11 课 · 数字键",
        desc: "数字键由手指向上伸出，注意左右手分工",
        lang: Lang::En,
        group: "en-lower",
        kind: LessonKind::Drill { keys: "1234567890" },
        len: 100,
    },
    Lesson {
        id: "words",
        title: "第 12 课 · 常用单词",
        desc: "把字母连成词，开始追求节奏",
        lang: Lang::En,
        group: "en-text",
        kind: LessonKind::Words {
            pool: WORDS,
            sep: " ",
        },
        len: 160,
    },
    Lesson {
        id: "shift",
        title: "第 13 课 · 大写与标点",
        desc: "大写用另一只手的小指按住 Shift，需要左右手配合",
        lang: Lang::En,
        group: "en-text",
        kind: LessonKind::Text { text: TEXT_SHIFT },
        len: 0,
    },
    Lesson {
        id: "passage",
        title: "第 14 课 · 综合文章",
        desc: "一段完整的英文，练到最后",
        lang: Lang::En,
        group: "en-text",
        kind: LessonKind::Text { text: TEXT_PASSAGE },
        len: 0,
    },
    Lesson {
        id: "zh-hanzi-1",
        title: "中文 · 常用字（一）",
        desc: "高频常用字，用系统输入法输入；打错的字会标红",
        lang: Lang::Zh,
        group: "zh-hanzi",
        kind: LessonKind::Chars { pool: HANZI_1 },
        len: 40,
    },
    Lesson {
        id: "zh-hanzi-2",
        title: "中文 · 常用字（二）",
        desc: "打字场景常用字",
        lang: Lang::Zh,
        group: "zh-hanzi",
        kind: LessonKind::Chars { pool: HANZI_2 },
        len: 40,
    },
    Lesson {
        id: "zh-words",
        title: "中文 · 常用词",
        desc: "两字词的连续输入，练词组节奏",
        lang: Lang::Zh,
        group: "zh-word",
        kind: LessonKind::Words {
            pool: CN_WORDS,
            sep: "",
        },
        len: 40,
    },
    Lesson {
        id: "zh-passage-1",
        title: "中文 · 短文（一）",
        desc: "带标点的完整句子，标点也算正确率",
        lang: Lang::Zh,
        group: "zh-text",
        kind: LessonKind::Text { text: CN_PASSAGE_1 },
        len: 0,
    },
    Lesson {
        id: "zh-passage-2",
        title: "中文 · 短文（二）",
        desc: "再来一段，练连续输入",
        lang: Lang::Zh,
        group: "zh-text",
        kind: LessonKind::Text { text: CN_PASSAGE_2 },
        len: 0,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drill_uses_only_lesson_keys() {
        let lesson = &LESSONS[0];
        let text = lesson.generate(42);
        assert!(text.chars().all(|c| c == ' ' || "fj".contains(c)));
        assert!(text.chars().filter(|c| *c != ' ').count() >= lesson.len);
    }

    #[test]
    fn generate_is_deterministic_and_seed_varied() {
        let lesson = &LESSONS[11];
        assert_eq!(lesson.generate(7), lesson.generate(7));
        assert_ne!(lesson.generate(7), lesson.generate(8));
    }

    #[test]
    fn english_lessons_are_ascii() {
        for lesson in LESSONS.iter().filter(|l| l.lang == Lang::En) {
            let text = lesson.generate(1);
            assert!(!text.is_empty(), "{} 生成了空文本", lesson.id);
            assert!(
                text.is_ascii(),
                "{} 含非 ASCII 字符，无法用美式键盘输入",
                lesson.id
            );
        }
    }

    #[test]
    fn chinese_lessons_have_no_ascii_or_whitespace() {
        for lesson in LESSONS.iter().filter(|l| l.lang == Lang::Zh) {
            let text = lesson.generate(1);
            assert!(!text.is_empty(), "{} 生成了空文本", lesson.id);
            assert!(
                text.chars().all(|c| !c.is_whitespace()),
                "{} 含空白字符：输入法里空格是选字键，打不出来",
                lesson.id
            );
            assert!(
                text.chars().all(|c| !c.is_ascii_alphanumeric()),
                "{} 含 ASCII 字母数字，汉字课程不应该有",
                lesson.id
            );
        }
    }

    #[test]
    fn chinese_char_drill_only_uses_pool() {
        let lesson = LESSONS.iter().find(|l| l.id == "zh-hanzi-1").unwrap();
        let pool: Vec<char> = HANZI_1.chars().collect();
        assert!(lesson.generate(3).chars().all(|c| pool.contains(&c)));
    }

    #[test]
    fn every_lesson_belongs_to_a_group_of_its_language() {
        for lesson in LESSONS {
            let group = GROUPS
                .iter()
                .find(|g| g.id == lesson.group)
                .unwrap_or_else(|| panic!("{} 的分组 {} 不存在", lesson.id, lesson.group));
            assert_eq!(group.lang, lesson.lang, "{} 的分组语言不一致", lesson.id);
        }
    }

    #[test]
    fn every_group_has_lessons() {
        for group in GROUPS {
            assert!(
                LESSONS.iter().any(|l| l.group == group.id),
                "分组 {} 下面没有课程",
                group.id
            );
        }
    }
}
