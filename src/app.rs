//! 应用外壳：英文/中文两个 Tab + 分层菜单（课程分组 / 练习 / 测试 / 素材）+ 练习与结算。
//!
//! 菜单类界面统一支持鼠标与键盘：两者共用同一个高亮游标，
//! ↑↓ 移动、Enter 进入、数字键直达、Esc 回上一层、←→/Tab 切换语言。

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

use crate::dom::{KeyListener, Ticker};
use crate::engine::ErrorMode;
use crate::history::history_view;
use crate::keyboard::virtual_keyboard;
use crate::layout::{self, Finger};
use crate::lessons::{Lang, GROUPS, LESSONS};
use crate::materials::{self, Material};
use crate::model::{CharState, Stats};
use crate::rng::{next_seed, Rng};
use crate::segment::{detect_lang, random_segment};
use crate::session::Session;
use crate::shuangpin;
use crate::storage::{self, Record};

/// 素材片段的目标长度（字符数）。
const SEGMENT_EN: usize = 200;
const SEGMENT_ZH: usize = 60;

/// 一次练习的来源：决定成绩归档到哪一项，以及结束后「再来一次」重复什么。
#[derive(Clone, PartialEq)]
enum Source {
    Lesson(usize),
    Material {
        lang: Lang,
        shuangpin: bool,
        /// 指定素材时固定用这一段，否则从素材池随机
        material_id: Option<String>,
    },
    Test {
        lang: Lang,
        secs: u32,
    },
}

impl Source {
    fn lang(&self) -> Lang {
        match self {
            Source::Lesson(index) => LESSONS[*index].lang,
            Source::Material { lang, .. } | Source::Test { lang, .. } => *lang,
        }
    }

    /// (归档 id, 显示名, 语言)
    fn meta(&self) -> (String, String, Lang) {
        match self {
            Source::Lesson(index) => {
                let lesson = &LESSONS[*index];
                (lesson.id.to_string(), lesson.title.to_string(), lesson.lang)
            }
            Source::Material {
                lang, shuangpin, ..
            } => (
                format!(
                    "{}-material{}",
                    lang_tag(*lang),
                    if *shuangpin { "-sp" } else { "" }
                ),
                format!(
                    "{}素材练习{}",
                    lang.name(),
                    if *shuangpin { "（双拼）" } else { "" }
                ),
                *lang,
            ),
            Source::Test { lang, secs } => (
                format!("{}-test-{}", lang_tag(*lang), secs),
                format!("{}限时测试 · {} 分钟", lang.name(), secs / 60),
                *lang,
            ),
        }
    }
}

fn lang_tag(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "en",
        Lang::Zh => "zh",
    }
}

#[derive(Clone, PartialEq)]
enum Route {
    Home,
    /// 课程分组下的课程列表
    Group(String),
    /// 素材管理
    Material,
    /// 粘贴文本新建素材
    NewMaterial,
    /// 选择测试时长
    Tests,
    Practice,
    Result(Finished),
    History,
}

#[derive(Clone, PartialEq)]
struct Finished {
    title: String,
    lang: Lang,
    stats: Stats,
    /// 错得最多的键（英文）/ 打错的字（中文）
    top_errors: Vec<(char, u32)>,
    best_cpm: f64,
    is_record: bool,
    shuangpin: bool,
}

#[derive(Clone, PartialEq)]
struct Item {
    label: String,
    hint: String,
    action: Action,
    /// 可删除的素材 id（键盘 Delete 用）
    delete: Option<String>,
}

#[derive(Clone, PartialEq)]
enum Action {
    Go(Route),
    Lesson(usize),
    /// 从素材池随机截一段开始练习
    Segment {
        shuangpin: bool,
    },
    /// 用指定素材开始练习
    PracticeMaterial(String),
    /// 限时测试（秒）
    Test(u32),
    Import,
    DeleteMaterial(String),
    SaveMaterial,
    Restart,
    Back,
}

fn item(label: impl Into<String>, hint: impl Into<String>, action: Action) -> Item {
    Item {
        label: label.into(),
        hint: hint.into(),
        action,
        delete: None,
    }
}

fn fmt_secs(secs: f64) -> String {
    if secs < 60.0 {
        format!("{secs:.1} 秒")
    } else {
        format!("{} 分 {:04.1} 秒", (secs / 60.0) as u32, secs % 60.0)
    }
}

fn user_material_id() -> String {
    format!("user-{}", js_sys::Date::now() as u64)
}

/// 从素材池里挑一段。素材池为空时退回到同语言的第一课。
fn draw_segment(all: &[Material], lang: Lang, seed: u64, shuangpin: bool) -> String {
    let target = if shuangpin || lang == Lang::Zh {
        SEGMENT_ZH
    } else {
        SEGMENT_EN
    };
    let pool: Vec<&Material> = all.iter().filter(|m| m.lang == lang).collect();
    let mut rng = Rng::new(seed);
    match rng.pick(&pool) {
        Some(material) => random_segment(&material.text, target, seed),
        None => LESSONS
            .iter()
            .find(|l| l.lang == lang)
            .map(|l| l.generate(seed))
            .unwrap_or_default(),
    }
}

fn best_hint(records: &[Record], id: &str, suffix: &str) -> String {
    match storage::best(records, id) {
        Some(best) => format!("{suffix} · 最佳 {:.0}", best.cpm),
        None => suffix.to_string(),
    }
}

#[component]
pub fn App() -> impl IntoView {
    let route = RwSignal::new(Route::Home);
    let cursor = RwSignal::new(0usize);
    let tab = RwSignal::new(Lang::En);
    let mode = RwSignal::new(ErrorMode::StopOnError);
    let session = RwSignal::new(Session::new("", Lang::En, ErrorMode::StopOnError));
    let source = RwSignal::new(Source::Material {
        lang: Lang::En,
        shuangpin: false,
        material_id: None,
    });
    let test_limit = RwSignal::new(None::<u32>);
    let shuangpin_mode = RwSignal::new(false);
    let records = RwSignal::new(storage::load());
    let user_materials = RwSignal::new(materials::load());
    // 随包素材（编译期内嵌）+ 用户素材合成一份，只在用户素材变化时重算，渲染时不重建池子
    let all_materials = Memo::new(move |_| {
        let mut all = materials::bundled();
        all.extend(user_materials.get());
        all
    });
    let name_input = RwSignal::new(String::new());
    let text_input = RwSignal::new(String::new());
    // 每次开始练习换一个种子，避免每次都是同一段
    let seed = RwSignal::new(0x9E37_79B9_7F4A_7C15u64);
    let now = RwSignal::new(js_sys::Date::now());
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let file_ref = NodeRef::<leptos::html::Input>::new();

    let stats = Memo::new(move |_| session.with(|s| s.stats(now.get())));
    let session_lang = move || source.with(|s| s.lang());
    let is_zh = move || session_lang() == Lang::Zh;

    // ---------- 中文课：输入框是唯一输入通道 ----------
    let focus_input = move || {
        if let Some(input) = input_ref.get() {
            let _ = input.focus();
        }
    };
    let clear_input = move || {
        if let Some(input) = input_ref.get() {
            input.set_value("");
        }
    };
    input_ref.on_load(move |input| {
        let _ = input.focus();
    });

    // ---------- 开始练习 ----------
    let start_lesson = move |index: usize| {
        seed.update(|s| *s = next_seed(*s));
        let lesson = &LESSONS[index];
        let text = lesson.generate(seed.get_untracked());
        session.set(Session::new(&text, lesson.lang, mode.get_untracked()));
        source.set(Source::Lesson(index));
        test_limit.set(None);
        shuangpin_mode.set(false);
        route.set(Route::Practice);
        cursor.set(0);
        if lesson.lang == Lang::Zh {
            clear_input();
            focus_input();
        }
    };

    let begin = move |text: String, src: Source, limit: Option<u32>| {
        let lang = src.lang();
        let shuangpin = matches!(
            src,
            Source::Material {
                shuangpin: true,
                ..
            }
        );
        session.set(Session::new(&text, lang, mode.get_untracked()));
        source.set(src);
        test_limit.set(limit);
        shuangpin_mode.set(shuangpin);
        route.set(Route::Practice);
        cursor.set(0);
        if lang == Lang::Zh {
            clear_input();
            focus_input();
        }
    };

    let start_segment = move |lang: Lang, shuangpin: bool, secs: Option<u32>| {
        seed.update(|s| *s = next_seed(*s));
        let text =
            all_materials.with(|all| draw_segment(all, lang, seed.get_untracked(), shuangpin));
        let src = match secs {
            Some(secs) => Source::Test { lang, secs },
            None => Source::Material {
                lang,
                shuangpin,
                material_id: None,
            },
        };
        begin(text, src, secs);
    };

    // 用指定素材开练；素材已被删掉时退回随机片段
    let start_material = move |id: String| {
        let found = all_materials.with(|all| all.iter().find(|m| m.id == id).cloned());
        match found {
            Some(material) => {
                seed.update(|s| *s = next_seed(*s));
                let target = if material.lang == Lang::Zh {
                    SEGMENT_ZH
                } else {
                    SEGMENT_EN
                };
                let text = random_segment(&material.text, target, seed.get_untracked());
                let src = Source::Material {
                    lang: material.lang,
                    shuangpin: false,
                    material_id: Some(material.id.clone()),
                };
                begin(text, src, None);
            }
            None => start_segment(tab.get_untracked(), false, None),
        }
    };

    // ---------- 结束与续段 ----------
    let finish = move || {
        let now_ms = js_sys::Date::now();
        let (id, title, lang) = source.with_untracked(|s| s.meta());
        let (stats, top_errors) = session.with(|s| (s.stats(now_ms), s.top_error_keys(3)));
        let previous_best = storage::best(&records.get_untracked(), &id)
            .map(|r| r.cpm)
            .unwrap_or(0.0);
        records.set(storage::push(Record {
            lesson_id: id,
            title: title.clone(),
            at_ms: now_ms,
            cpm: stats.cpm,
            accuracy: stats.accuracy,
            errors: stats.errors,
            chars: stats.correct,
            secs: stats.secs,
        }));
        route.set(Route::Result(Finished {
            title,
            lang,
            stats,
            top_errors,
            best_cpm: previous_best.max(stats.cpm),
            is_record: stats.cpm > previous_best,
            shuangpin: shuangpin_mode.get_untracked(),
        }));
        cursor.set(0);
    };

    // 限时测试：快打完时自动续上新片段，时间到才结算
    let maybe_extend = move || {
        if test_limit.get_untracked().is_none() {
            return;
        }
        let (position, len) = session.with_untracked(|s| (s.cursor(), s.len()));
        if position + 60 < len {
            return;
        }
        seed.update(|s| *s = next_seed(*s));
        let lang = session_lang();
        let text = all_materials.with(|all| draw_segment(all, lang, seed.get_untracked(), false));
        session.update(|s| s.extend_target(&text));
    };

    // 心跳：驱动用时显示；限时测试到点自动结算
    let _ticker = StoredValue::new_local(Ticker::start(100, move || {
        let now_ms = js_sys::Date::now();
        now.set(now_ms);
        if route.get_untracked() != Route::Practice {
            return;
        }
        if let Some(limit) = test_limit.get_untracked() {
            if session.with_untracked(|s| s.stats(now_ms).secs) >= limit as f64 {
                finish();
            }
        }
    }));

    // 中文：输入法提交文字后整体刷新
    let sync_from_input = move || {
        let Some(input) = input_ref.get() else {
            return;
        };
        let text = input.value();
        let now_ms = js_sys::Date::now();
        session.update(|s| s.sync_text(&text, now_ms));
        if session.with_untracked(|s| s.finished()) {
            if test_limit.get_untracked().is_some() {
                maybe_extend();
            } else {
                finish();
            }
        }
    };
    let on_cn_input = move |ev: web_sys::Event| {
        // 组词中的内容是预编辑文本，还没提交，不参与判定
        let composing = ev
            .dyn_ref::<web_sys::InputEvent>()
            .is_some_and(web_sys::InputEvent::is_composing);
        if !composing {
            sync_from_input();
        }
    };
    let on_cn_commit = move |_ev: web_sys::CompositionEvent| sync_from_input();

    // ---------- 素材导入 ----------
    let on_files = move |_ev: web_sys::Event| {
        let Some(input) = file_ref.get() else {
            return;
        };
        let Some(files) = input.files() else {
            return;
        };
        for index in 0..files.length() {
            let Some(file) = files.get(index) else {
                continue;
            };
            let name = file.name();
            let Ok(reader) = web_sys::FileReader::new() else {
                continue;
            };
            let done_reader = reader.clone();
            let on_done: Closure<dyn FnMut()> = Closure::new(move || {
                let Ok(result) = done_reader.result() else {
                    return;
                };
                let Some(text) = result.as_string() else {
                    return;
                };
                if text.trim().is_empty() {
                    return;
                }
                // 按内容归类，而不是按当前 tab：否则会出现「中文课里挂英文素材」这种打不出来的组合
                let lang = detect_lang(&text);
                user_materials.set(materials::push(Material {
                    id: user_material_id(),
                    name: name.clone(),
                    lang,
                    text,
                    bundled: false,
                }));
                // 切到对应语言，让导入结果立刻可见
                tab.set(lang);
            });
            reader.set_onloadend(Some(on_done.as_ref().unchecked_ref()));
            // 一次性回调：导入是低频操作，读完就丢给浏览器回收
            on_done.forget();
            let _ = reader.read_as_text(&file);
        }
        // 清空，方便重复导入同一个文件
        input.set_value("");
    };

    // ---------- 导航 ----------
    let back = move || {
        let target = match route.get_untracked() {
            Route::NewMaterial => Route::Material,
            Route::Home | Route::Practice => Route::Home,
            _ => Route::Home,
        };
        route.set(target);
        cursor.set(0);
    };

    let activate = move |action: Action| match action {
        Action::Go(target) => {
            route.set(target);
            cursor.set(0);
        }
        Action::Lesson(index) => start_lesson(index),
        Action::Segment { shuangpin } => start_segment(tab.get_untracked(), shuangpin, None),
        Action::PracticeMaterial(id) => start_material(id),
        Action::Test(secs) => start_segment(tab.get_untracked(), false, Some(secs)),
        Action::Import => {
            if let Some(input) = file_ref.get() {
                input.click();
            }
        }
        Action::DeleteMaterial(id) => user_materials.set(materials::remove(&id)),
        Action::SaveMaterial => {
            let name = name_input.get_untracked().trim().to_string();
            let text = text_input.get_untracked();
            if !text.trim().is_empty() {
                // 同导入：按内容归类并切到对应语言
                let lang = detect_lang(&text);
                let name = if name.is_empty() {
                    format!(
                        "{}素材 {}",
                        lang.name(),
                        user_materials.get_untracked().len() + 1
                    )
                } else {
                    name
                };
                user_materials.set(materials::push(Material {
                    id: user_material_id(),
                    name,
                    lang,
                    text,
                    bundled: false,
                }));
                tab.set(lang);
                name_input.set(String::new());
                text_input.set(String::new());
            }
            route.set(Route::Material);
            cursor.set(0);
        }
        Action::Restart => match source.get_untracked() {
            Source::Lesson(index) => start_lesson(index),
            Source::Material {
                lang,
                shuangpin,
                material_id,
            } => match material_id {
                Some(id) => start_material(id),
                None => start_segment(lang, shuangpin, None),
            },
            Source::Test { lang, secs } => start_segment(lang, false, Some(secs)),
        },
        Action::Back => back(),
    };

    // ---------- 菜单项（渲染与键盘共用同一份） ----------
    let items = Memo::new(move |_| {
        let lang = tab.get();
        let records_snapshot = records.get();
        match route.get() {
            Route::Home => {
                let mut items: Vec<Item> = GROUPS
                    .iter()
                    .filter(|group| group.lang == lang)
                    .map(|group| {
                        let count = LESSONS.iter().filter(|l| l.group == group.id).count();
                        item(
                            group.title,
                            format!("{count} 课"),
                            Action::Go(Route::Group(group.id.to_string())),
                        )
                    })
                    .collect();
                if lang == Lang::Zh {
                    items.push(item(
                        "拼音练习",
                        "用系统输入法打素材片段，打完即结算",
                        Action::Segment { shuangpin: false },
                    ));
                    items.push(item(
                        "双拼练习（小鹤）",
                        "输入法切到小鹤方案，配键位表练习",
                        Action::Segment { shuangpin: true },
                    ));
                } else {
                    items.push(item(
                        "练习",
                        "从英文素材里随机截一段，打完即结算",
                        Action::Segment { shuangpin: false },
                    ));
                }
                items.push(item(
                    "测试",
                    "限时，时间到自动结算",
                    Action::Go(Route::Tests),
                ));
                items.push(item(
                    "素材",
                    "导入 txt 或粘贴文本，练习与测试都从素材取片段",
                    Action::Go(Route::Material),
                ));
                items.push(item(
                    "成绩档案",
                    "每次练习的记录与速度趋势",
                    Action::Go(Route::History),
                ));
                items
            }
            Route::Group(group) => LESSONS
                .iter()
                .enumerate()
                .filter(|(_, lesson)| lesson.group == group)
                .map(|(index, lesson)| {
                    item(
                        lesson.title,
                        best_hint(&records_snapshot, lesson.id, lesson.desc),
                        Action::Lesson(index),
                    )
                })
                .collect(),
            Route::Tests => vec![
                item("1 分钟测试", "计 1 分钟，到点结算", Action::Test(60)),
                item("2 分钟测试", "计 2 分钟，到点结算", Action::Test(120)),
                item("5 分钟测试", "计 5 分钟，到点结算", Action::Test(300)),
            ],
            Route::Material => {
                let mut items = vec![
                    item(
                        "导入 .txt 文件",
                        "可多选，按内容自动归入中文/英文",
                        Action::Import,
                    ),
                    item(
                        "粘贴文本新建",
                        "手动写名字和正文",
                        Action::Go(Route::NewMaterial),
                    ),
                ];
                let materials_snapshot = all_materials.get();
                for material in materials_snapshot.iter().filter(|m| m.lang == lang) {
                    let mut entry = item(
                        material.name.clone(),
                        format!(
                            "{} 字{}",
                            material.len_chars(),
                            if material.bundled {
                                " · 随包素材"
                            } else {
                                " · 回车开始练习，Delete 删除"
                            }
                        ),
                        Action::PracticeMaterial(material.id.clone()),
                    );
                    if !material.bundled {
                        entry.delete = Some(material.id.clone());
                    }
                    items.push(entry);
                }
                items
            }
            Route::Result(_) => vec![
                item("再来一次", "换一段继续", Action::Restart),
                item("返回课程表", "回到菜单", Action::Back),
            ],
            Route::History => vec![],
            Route::Practice | Route::NewMaterial => vec![],
        }
    });

    // ---------- 键盘 ----------
    let _listener = StoredValue::new_local(KeyListener::new(move |ev: web_sys::KeyboardEvent| {
        if ev.ctrl_key() || ev.meta_key() || ev.alt_key() {
            return;
        }
        let key = ev.key();

        if route.get_untracked() == Route::Practice {
            match key.as_str() {
                "Escape" => back(),
                "Backspace" => {
                    if !is_zh() {
                        ev.prevent_default();
                        session.update(|s| s.backspace());
                    }
                }
                _ => {
                    // 中文与双拼的字符交给输入法和输入框
                    if is_zh() {
                        return;
                    }
                    let mut chars = key.chars();
                    if let (Some(ch), None) = (chars.next(), chars.next()) {
                        if !(ch.is_ascii_graphic() || ch == ' ') {
                            return;
                        }
                        if session.with_untracked(|s| s.is_empty() || s.finished()) {
                            return;
                        }
                        ev.prevent_default();
                        let now_ms = js_sys::Date::now();
                        session.update(|s| s.press_char(ch, now_ms));
                        if session.with_untracked(|s| s.finished()) {
                            if test_limit.get_untracked().is_some() {
                                maybe_extend();
                            } else {
                                finish();
                            }
                        }
                    }
                }
            }
            return;
        }

        if route.get_untracked() == Route::NewMaterial {
            if key == "Escape" {
                back();
            }
            return;
        }

        let list = items.get_untracked();
        match key.as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                if !list.is_empty() {
                    cursor.update(|c| *c = (*c + 1) % list.len());
                }
            }
            "ArrowUp" => {
                ev.prevent_default();
                if !list.is_empty() {
                    cursor.update(|c| *c = (*c + list.len() - 1) % list.len());
                }
            }
            "ArrowLeft" | "ArrowRight" | "Tab" => {
                ev.prevent_default();
                let next = if tab.get_untracked() == Lang::En {
                    Lang::Zh
                } else {
                    Lang::En
                };
                tab.set(next);
                route.set(Route::Home);
                cursor.set(0);
            }
            "Enter" | " " => {
                ev.prevent_default();
                if let Some(entry) = list.get(cursor.get_untracked()) {
                    activate(entry.action.clone());
                }
            }
            "Escape" => back(),
            "Delete" => {
                if let Some(id) = list
                    .get(cursor.get_untracked())
                    .and_then(|entry| entry.delete.clone())
                {
                    activate(Action::DeleteMaterial(id));
                }
            }
            "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                let index = key.parse::<usize>().unwrap_or(1) - 1;
                if let Some(entry) = list.get(index) {
                    activate(entry.action.clone());
                }
            }
            _ => {}
        }
    }));

    // ---------- 视图 ----------
    let hint = move || {
        let next = session.with(|s| s.expected());
        match next {
            None => "本段内容已打完".to_string(),
            Some(ch) if is_zh() => format!("下一个字：{ch}"),
            Some(' ') => "下一个：空格 · 拇指".to_string(),
            Some(ch) => {
                let finger = layout::finger_of(ch)
                    .map(layout::finger_name)
                    .unwrap_or("未知键");
                let shift = if layout::needs_shift(ch) {
                    "，另一只手按住 Shift"
                } else {
                    ""
                };
                format!("下一个：{ch} · {finger}{shift}")
            }
        }
    };

    let text_spans = move || {
        session.with(|s| {
            let cursor = s.cursor();
            let mistake = s.mistake_at();
            s.target()
                .iter()
                .enumerate()
                .map(|(i, &ch)| {
                    let mut class = String::from("ch");
                    match s.state_at(i) {
                        CharState::Todo => {}
                        CharState::Done => class.push_str(" done"),
                        CharState::Wrong => class.push_str(" wrong"),
                    }
                    if mistake == Some(i) {
                        class.push_str(" flash");
                    } else if i == cursor {
                        class.push_str(" cursor");
                    }
                    view! { <span class=class>{ch.to_string()}</span> }
                })
                .collect_view()
        })
    };

    let menu = move || {
        let list = items.get();
        if list.is_empty() {
            return ().into_any();
        }
        list.into_iter()
            .enumerate()
            .map(|(index, entry)| {
                let action = entry.action.clone();
                let delete_id = entry.delete.clone();
                view! {
                    <button
                        class=move || if cursor.get() == index { "item sel" } else { "item" }
                        on:mouseenter=move |_| cursor.set(index)
                        on:click=move |_| activate(action.clone())
                    >
                        <span class="item-index">{format!("{}", index + 1)}</span>
                        <span class="item-label">{entry.label.clone()}</span>
                        <span class="item-hint">{entry.hint.clone()}</span>
                        {delete_id.map(|id| {
                            view! {
                                <span
                                    class="item-delete"
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        activate(Action::DeleteMaterial(id.clone()));
                                    }
                                >
                                    "✕"
                                </span>
                            }
                        })}
                    </button>
                }
            })
            .collect_view()
            .into_any()
    };

    view! {
        <header class="topbar">
            <h1>"TT 打字训练"</h1>
            <div class="topbar-right">
                {move || {
                    if route.get() == Route::Practice {
                        view! { <span class="sub">"Esc 退出练习"</span> }.into_any()
                    } else {
                        view! {
                            <div class="tabs">
                                {[Lang::En, Lang::Zh]
                                    .into_iter()
                                    .map(|l| {
                                        view! {
                                            <button
                                                class=move || {
                                                    if tab.get() == l { "tab on" } else { "tab" }
                                                }
                                                on:click=move |_| {
                                                    tab.set(l);
                                                    route.set(Route::Home);
                                                    cursor.set(0);
                                                }
                                            >
                                                {l.name()}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                            {move || {
                                if route.get() == Route::Home {
                                    view! { <span></span> }.into_any()
                                } else {
                                    view! {
                                        <button class="btn ghost" on:click=move |_| back()>
                                            "返回"
                                        </button>
                                    }
                                        .into_any()
                                }
                            }}
                        }
                            .into_any()
                    }
                }}
            </div>
        </header>

        <main class="screen">
            {move || match route.get() {
                Route::Home | Route::Group(_) | Route::Tests | Route::Material => {
                    view! {
                        <section class="menu">
                            {move || {
                                if route.get() == Route::Material {
                                    view! {
                                        <p class="lede">
                                            "练习与测试都会从这里的素材里随机截取片段。导入的 txt 存"
                                            "在本地，不会上传。"
                                        </p>
                                    }
                                        .into_any()
                                } else if route.get() == Route::Tests {
                                    view! {
                                        <p class="lede">
                                            "限时测试：到点自动结算，速度按打对的字符算。"
                                        </p>
                                    }
                                        .into_any()
                                } else {
                                    view! { <span></span> }.into_any()
                                }
                            }}
                            <div class="items">{menu}</div>
                        </section>
                    }
                        .into_any()
                }

                Route::NewMaterial => {
                    view! {
                        <section class="new-material">
                            <h2 class="section-title">
                                {move || format!("粘贴文本新建素材（{}）", tab.get().name())}
                            </h2>
                            <input
                                class="name-input"
                                placeholder="素材名字"
                                prop:value=move || name_input.get()
                                on:input=move |ev: web_sys::Event| {
                                    if let Some(input) = ev
                                        .target()
                                        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                    {
                                        name_input.set(input.value());
                                    }
                                }
                            />
                            <textarea
                                class="text-input"
                                placeholder="把要练的文本粘贴到这里"
                                prop:value=move || text_input.get()
                                on:input=move |ev: web_sys::Event| {
                                    if let Some(area) = ev
                                        .target()
                                        .and_then(|t| {
                                            t.dyn_into::<web_sys::HtmlTextAreaElement>().ok()
                                        })
                                    {
                                        text_input.set(area.value());
                                    }
                                }
                            ></textarea>
                            <div class="controls">
                                <button
                                    class="btn"
                                    on:click=move |_| activate(Action::SaveMaterial)
                                >
                                    "保存素材"
                                </button>
                                <button class="btn ghost" on:click=move |_| back()>
                                    "取消"
                                </button>
                            </div>
                        </section>
                    }
                        .into_any()
                }

                Route::History => {
                    history_view(records, move || back()).into_any()
                }

                Route::Practice => {
                    view! {
                        <section class="practice" on:click=move |_| focus_input()>
                            <div class="hud">
                                <div class="cell">
                                    <span class="k">"当前"</span>
                                    <b>{move || {
                                        let (_, title, _) = source.with(|s| s.meta());
                                        title
                                    }}</b>
                                </div>
                                <div class="cell">
                                    <span class="k">
                                        {move || if is_zh() { "速度 (字/分)" } else { "速度 (CPM)" }}
                                    </span>
                                    <b>{move || format!("{:.0}", stats.get().cpm)}</b>
                                </div>
                                <div class="cell">
                                    <span class="k">"正确率"</span>
                                    <b>{move || format!("{:.1}%", stats.get().accuracy)}</b>
                                </div>
                                <div class="cell">
                                    <span class="k">
                                        {move || if is_zh() { "错字" } else { "错误" }}
                                    </span>
                                    <b>{move || stats.get().errors.to_string()}</b>
                                </div>
                                <div class="cell">
                                    <span class="k">
                                        {move || {
                                            if test_limit.get().is_some() { "剩余" } else { "用时" }
                                        }}
                                    </span>
                                    <b>{move || {
                                        let secs = stats.get().secs;
                                        match test_limit.get() {
                                            Some(limit) => fmt_secs((limit as f64 - secs).max(0.0)),
                                            None => fmt_secs(secs),
                                        }
                                    }}</b>
                                </div>
                                <div class="cell">
                                    <span class="k">"进度"</span>
                                    <b>{move || {
                                        let s = stats.get();
                                        format!("{}/{}", s.cursor, s.total)
                                    }}</b>
                                </div>
                            </div>

                            <div class="progress">
                                <i style=move || {
                                    let s = stats.get();
                                    let percent = if s.total == 0 {
                                        0.0
                                    } else {
                                        s.cursor as f64 / s.total as f64 * 100.0
                                    };
                                    format!("width:{percent:.1}%")
                                }></i>
                            </div>

                            <div class=move || if is_zh() { "text zh" } else { "text" }>{text_spans}</div>

                            <div class="hint">{hint}</div>

                            {move || if is_zh() {
                                view! {
                                    <div class="cn-panel">
                                        <input
                                            node_ref=input_ref
                                            placeholder="在这里用输入法打字"
                                            on:input=on_cn_input
                                            on:compositionend=on_cn_commit
                                        />
                                        {move || {
                                            if shuangpin_mode.get() {
                                                view! { <div class="sp-ref">{shuangpin_ref()}</div> }
                                                    .into_any()
                                            } else {
                                                view! {
                                                    <p class="cn-tip">
                                                        "用系统输入法输入上面的文字；打错的字会标红，改对即可"
                                                    </p>
                                                }
                                                    .into_any()
                                            }
                                        }}
                                    </div>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <div class="keys-panel">
                                        {virtual_keyboard(move || session.with(|s| s.expected()))}
                                        <div class="legend">
                                            {[
                                                Finger::LPinky,
                                                Finger::LRing,
                                                Finger::LMiddle,
                                                Finger::LIndex,
                                                Finger::Thumb,
                                                Finger::RIndex,
                                                Finger::RMiddle,
                                                Finger::RRing,
                                                Finger::RPinky,
                                            ]
                                                .into_iter()
                                                .map(|f| {
                                                    view! {
                                                        <span class=format!(
                                                            "legend-item {}",
                                                            layout::finger_class(f),
                                                        )>{layout::finger_name(f)}</span>
                                                    }
                                                })
                                                .collect_view()}
                                            <span class="legend-item">"Esc 退出练习"</span>
                                        </div>
                                    </div>
                                }
                                    .into_any()
                            }}

                            <div class="controls">
                                <button
                                    class="btn"
                                    on:click=move |_| activate(Action::Restart)
                                >
                                    "重新开始"
                                </button>
                                <button class="btn ghost" on:click=move |_| back()>
                                    "返回菜单"
                                </button>
                            </div>
                        </section>
                    }
                        .into_any()
                }

                Route::Result(finished) => {
                    let stats = finished.stats;
                    let zh = finished.lang == Lang::Zh;
                    let cells: Vec<(&'static str, String)> = if zh {
                        vec![
                            ("正确率", format!("{:.1}%", stats.accuracy)),
                            ("输入 / 错字", format!("{} / {}", stats.strokes, stats.errors)),
                            ("用时", fmt_secs(stats.secs)),
                            ("正确字数", format!("{}/{}", stats.correct, stats.total)),
                            ("本项最佳", format!("{:.0} 字/分", finished.best_cpm)),
                        ]
                    } else {
                        vec![
                            ("WPM", format!("{:.1}", stats.wpm)),
                            ("正确率", format!("{:.1}%", stats.accuracy)),
                            ("击键 / 错误", format!("{} / {}", stats.strokes, stats.errors)),
                            ("用时", fmt_secs(stats.secs)),
                            ("正确字符", format!("{}/{}", stats.correct, stats.total)),
                            ("本项最佳", format!("{:.0} CPM", finished.best_cpm)),
                        ]
                    };
                    let problem_items = if finished.top_errors.is_empty() {
                        "无".to_string()
                    } else {
                        finished
                            .top_errors
                            .iter()
                            .map(|(ch, times)| format!("{ch} ×{times}"))
                            .collect::<Vec<_>>()
                            .join("   ")
                    };
                    let problem_label = if zh { "错字" } else { "问题键" };
                    let title = finished.title.clone();
                    let mode_note = if finished.shuangpin {
                        "双拼练习 · 用输入法的小鹤方案输入"
                    } else {
                        ""
                    };

                    view! {
                        <section class="result">
                            <h2>{if finished.is_record { "新纪录" } else { "练习完成" }}</h2>
                            <p class="result-title">{title}</p>
                            <div class="score">
                                {format!("{:.0}", stats.cpm)}
                                <span>{if zh { " 字/分" } else { " CPM" }}</span>
                            </div>
                            <p class="lede">
                                {if zh {
                                    "净速度，只计打对的字"
                                } else {
                                    "净速度，只计正确字符；5 字符算 1 词"
                                }}
                            </p>

                            <div class="hud">
                                {cells
                                    .into_iter()
                                    .map(|(label, value)| {
                                        view! {
                                            <div class="cell">
                                                <span class="k">{label}</span>
                                                <b>{value}</b>
                                            </div>
                                        }
                                    })
                                    .collect_view()}
                            </div>

                            <div class="problem">
                                <span class="k">{problem_label}</span>
                                <b>{problem_items}</b>
                                <span class="k">{mode_note}</span>
                            </div>

                            <div class="items">{menu}</div>
                        </section>
                    }
                        .into_any()
                }
            }}
        </main>

        <input
            node_ref=file_ref
            type="file"
            accept=".txt,text/plain"
            multiple
            style="display:none"
            on:change=on_files
        />
    }
}

/// 小鹤双拼键位参考表。
fn shuangpin_ref() -> impl IntoView {
    view! {
        <p class="sp-tip">
            "小鹤双拼：声母除 zh → V、ch → I、sh → U 外，其余就是对应字母键；韵母一个占一个键。"
            "零声母音节（安 an、爱 ai 等）通常把首字母当声母处理。以你输入法里的方案为准。"
        </p>
        <div class="sp-rows">
            {shuangpin::ROWS
                .iter()
                .map(|row| {
                    view! {
                        <div class="sp-row">
                            {row
                                .chars()
                                .map(|key| match shuangpin::key_at(key) {
                                    Some(entry) => {
                                        view! {
                                            <span class="sp-key">
                                                <em>{entry.mnemonic.to_string()}</em>
                                                <b>{entry.key.to_string()}</b>
                                                <i>{entry.yunmu.to_string()}</i>
                                                {if entry.initial.is_empty() {
                                                    view! { <small></small> }.into_any()
                                                } else {
                                                    view! {
                                                        <small>{format!("兼 {}", entry.initial)}</small>
                                                    }
                                                        .into_any()
                                                }}
                                            </span>
                                        }
                                            .into_any()
                                    }
                                    None => view! { <span></span> }.into_any(),
                                })
                                .collect_view()}
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}
