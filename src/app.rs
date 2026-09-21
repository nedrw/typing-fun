//! 应用外壳：英文/中文两个 Tab + 分层菜单（课程分组 / 练习 / 测试 / 素材）+ 练习与结算。
//!
//! 菜单类界面统一支持鼠标与键盘：两者共用同一个高亮游标，
//! ↑↓ 移动、Enter 进入、数字键直达、Esc 回上一层、←→/Tab 切换语言。

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

use crate::dom::{CompositionListener, KeyListener, Ticker};
use crate::engine::{ErrorMode, KeyResult};
use crate::heat;
use crate::history::history_view;
use crate::keyboard::virtual_keyboard;
use crate::layout::{self, Finger};
use crate::lessons::{Lang, GROUPS, LESSONS};
use crate::materials::{self, Material};
use crate::model::{CharState, Stats};
use crate::rng::{next_seed, Rng};
use crate::segment::{detect_lang, random_segment};
use crate::session::Session;
use crate::settings::Settings;
use crate::shuangpin;
use crate::storage::{self, Record};
use crate::store;

/// 素材片段的目标长度（字符数）。
const SEGMENT_EN: usize = 200;
const SEGMENT_ZH: usize = 60;

/// 数据文件名（相对 app data dir）。
const RECORDS_FILE: &str = storage::FILE;
const MATERIALS_FILE: &str = materials::FILE;
const SETTINGS_FILE: &str = crate::settings::FILE;

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
    /// 双拼按键判定（小鹤）：secs 为 Some 时是限时测试
    Shuangpin {
        secs: Option<u32>,
        /// 指定素材时固定用这一段，否则从素材池随机
        material_id: Option<String>,
    },
}

impl Source {
    fn lang(&self) -> Lang {
        match self {
            Source::Lesson(index) => LESSONS[*index].lang,
            Source::Material { lang, .. } | Source::Test { lang, .. } => *lang,
            Source::Shuangpin { .. } => Lang::Zh,
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
            Source::Shuangpin { secs, .. } => match secs {
                Some(secs) => (
                    format!("zh-shuangpin-test-{secs}"),
                    format!("双拼键位测试 · {} 分钟", secs / 60),
                    Lang::Zh,
                ),
                None => (
                    "zh-shuangpin-key".to_string(),
                    "双拼键位练习".to_string(),
                    Lang::Zh,
                ),
            },
        }
    }
}

fn lang_tag(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "en",
        Lang::Zh => "zh",
    }
}

/// 最近一次中文练习的判定方式：素材页按它决定点素材后进哪个引擎。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ZhMode {
    /// 输入法判定：拼音练习、跟输入法练双拼、中文课程与测试
    Ime,
    /// 小鹤双拼按键判定
    Shuangpin,
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
    /// 双拼按键判定模式（错误统计的是击键，不是错字）
    sp: bool,
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
    /// 双拼按键判定（小鹤）：secs 为 Some 时是限时测试
    Shuangpin {
        secs: Option<u32>,
    },
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

/// 把文件内容解成文本：先按 UTF-8 严格解，失败再按 GB18030 试。
///
/// 中文用户手头的 txt 很多是 Windows 记事本存的 GBK/GB18030，
/// 直接按 UTF-8 松解码会得到一堆替换字符。
fn decode_text(buffer: &js_sys::ArrayBuffer) -> Option<String> {
    let strict = web_sys::TextDecoderOptions::new();
    strict.set_fatal(true);
    if let Ok(decoder) = web_sys::TextDecoder::new_with_label_and_options("utf-8", &strict) {
        if let Ok(text) = decoder.decode_with_buffer_source(buffer) {
            return Some(text);
        }
    }
    web_sys::TextDecoder::new_with_label("gb18030")
        .ok()?
        .decode_with_buffer_source(buffer)
        .ok()
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
        Some(material) => random_segment(&material.text, lang, target, seed),
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
    // 最近一次中文练习的模式：决定素材页点素材后进哪个引擎
    let zh_mode = RwSignal::new(ZhMode::Ime);
    let session = RwSignal::new(Session::new("", Lang::En, ErrorMode::StopOnError));
    let source = RwSignal::new(Source::Material {
        lang: Lang::En,
        shuangpin: false,
        material_id: None,
    });
    let test_limit = RwSignal::new(None::<u32>);
    let shuangpin_mode = RwSignal::new(false);
    let records = RwSignal::new(Vec::<Record>::new());
    let user_materials = RwSignal::new(Vec::<Material>::new());
    let storage_error = RwSignal::new(None::<String>);
    // 随包素材（编译期内嵌）+ 用户素材合成一份，只在用户素材变化时重算，渲染时不重建池子
    let all_materials = Memo::new(move |_| {
        let mut all = materials::bundled();
        all.extend(user_materials.get());
        all
    });

    // ---------- 用户数据落盘 ----------
    // 桌面端写 app data dir（RON），浏览器里退回 localStorage；失败在顶部横幅提示
    let on_storage_error = move |message: String| storage_error.set(Some(message));

    store::load::<Vec<Record>>(RECORDS_FILE, move |loaded| {
        if let Some(loaded) = loaded {
            records.set(storage::trim(loaded));
        }
    });
    store::load::<Vec<Material>>(MATERIALS_FILE, move |loaded| {
        if let Some(loaded) = loaded {
            user_materials.set(loaded);
        }
    });
    store::load::<Settings>(SETTINGS_FILE, move |loaded| {
        if let Some(loaded) = loaded {
            tab.set(loaded.lang);
        }
    });

    let persist_records = move |next: Vec<Record>| {
        records.set(next.clone());
        store::save(RECORDS_FILE, &next, on_storage_error);
    };
    let persist_materials = move |next: Vec<Material>| {
        user_materials.set(next.clone());
        store::save(MATERIALS_FILE, &next, on_storage_error);
    };
    let persist_tab = move |lang: Lang| {
        tab.set(lang);
        store::save(SETTINGS_FILE, &Settings { lang }, on_storage_error);
    };
    let name_input = RwSignal::new(String::new());
    let text_input = RwSignal::new(String::new());
    // 英文练习里检测到输入法组词时给出的提示
    let ime_notice = RwSignal::new(false);
    // 每次开始练习换一个种子，避免每次都是同一段
    let seed = RwSignal::new(0x9E37_79B9_7F4A_7C15u64);
    let now = RwSignal::new(js_sys::Date::now());
    // 火力条：打字蓄力、随时间衰减；爆发强度用最近 2.5 秒的即时速度
    let heat = RwSignal::new(0.0f64);
    // 爆发状态：满格进入，跌破 BURST_EXIT 才退出（滞回）
    let burst = RwSignal::new(false);
    let speed = RwSignal::new(heat::SpeedWindow::new(2_500.0));
    let input_ref = NodeRef::<leptos::html::Input>::new();
    let file_ref = NodeRef::<leptos::html::Input>::new();
    let text_ref = NodeRef::<leptos::html::Div>::new();

    let stats = Memo::new(move |_| session.with(|s| s.stats(now.get())));
    let session_lang = move || source.with(|s| s.lang());
    let is_zh = move || session_lang() == Lang::Zh;
    // 双拼按键判定：不用输入框，字母键直接进击键引擎
    let is_sp = move || matches!(source.get(), Source::Shuangpin { .. });
    // 只有中文输入法模式需要输入框
    let uses_input_field = move || is_zh() && !is_sp();
    // 中英文速度量级不同（英文按击键、中文按字、双拼一字两键），手感参数按模式选
    let heat_profile = move || {
        if is_sp() {
            heat::SP
        } else if is_zh() {
            heat::ZH
        } else {
            heat::EN
        }
    };
    let add_heat = move |correct: usize, mistakes: usize, at_ms: f64| {
        let profile = heat_profile();
        if correct > 0 {
            speed.update(|w| {
                for _ in 0..correct {
                    w.note(at_ms);
                }
            });
        }
        heat.update(|h| *h = profile.charge(*h, correct, mistakes));
        if heat.with_untracked(|h| heat::is_full(*h)) {
            burst.set(true);
        }
    };
    let reset_heat = move || {
        heat.set(0.0);
        burst.set(false);
        speed.update(|w| w.clear());
    };

    // 英文练习里检测到输入法组词：说明用户在中文输入法下敲键，提示切回英文键盘
    let _composition = StoredValue::new_local(CompositionListener::new(move |_| {
        if route.get_untracked() == Route::Practice && !is_zh() {
            ime_notice.set(true);
        }
    }));

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

    // on_load 只在节点首次挂载时触发；纯键盘导航进入练习不会有点击事件，
    // 靠这个 Effect 在路由切到练习页后把焦点放进输入框（每次进入都生效）
    Effect::new(move |_| {
        if route.get() == Route::Practice && uses_input_field() {
            if let Some(input) = input_ref.get() {
                let _ = input.focus();
            }
        }
    });

    // ---------- 开始练习 ----------
    let start_lesson = move |index: usize| {
        seed.update(|s| *s = next_seed(*s));
        let lesson = &LESSONS[index];
        let text = lesson.generate(seed.get_untracked());
        // 指法课要求打对当前字符才能前进
        session.set(Session::new(&text, lesson.lang, ErrorMode::StopOnError));
        ime_notice.set(false);
        reset_heat();
        if lesson.lang == Lang::Zh {
            zh_mode.set(ZhMode::Ime);
        }
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
        // 素材练习与限时测试面向速度：打错照常前进、可退格修正；
        // 指法课则要求打对当前字符（见 start_lesson）
        let error_mode = if matches!(src, Source::Lesson(_)) {
            ErrorMode::StopOnError
        } else {
            ErrorMode::Continue
        };
        session.set(Session::new(&text, lang, error_mode));
        ime_notice.set(false);
        reset_heat();
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
        if lang == Lang::Zh {
            zh_mode.set(ZhMode::Ime);
        }
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
                let text =
                    random_segment(&material.text, material.lang, target, seed.get_untracked());
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

    // 双拼按键判定：默认从中文素材池随机截一段；指定素材时用该素材
    let start_shuangpin = move |secs: Option<u32>, material_id: Option<String>| {
        seed.update(|s| *s = next_seed(*s));
        let text = all_materials.with(|all| match &material_id {
            Some(id) => all
                .iter()
                .find(|m| m.id == *id)
                .map(|m| random_segment(&m.text, Lang::Zh, SEGMENT_ZH, seed.get_untracked()))
                .unwrap_or_else(|| draw_segment(all, Lang::Zh, seed.get_untracked(), false)),
            None => draw_segment(all, Lang::Zh, seed.get_untracked(), false),
        });
        zh_mode.set(ZhMode::Shuangpin);
        // 面向速度：打错自动补上期望键继续，错误计入正确率
        session.set(Session::shuangpin(&text, ErrorMode::Continue));
        ime_notice.set(false);
        reset_heat();
        source.set(Source::Shuangpin { secs, material_id });
        test_limit.set(secs);
        shuangpin_mode.set(false);
        route.set(Route::Practice);
        cursor.set(0);
    };

    // ---------- 结束与续段 ----------
    let finish = move || {
        let now_ms = js_sys::Date::now();
        let (id, title, lang) = source.with_untracked(|s| s.meta());
        let (stats, top_errors, record_errors) =
            session.with(|s| (s.stats(now_ms), s.top_error_keys(3), s.top_error_keys(10)));
        let previous_best = storage::best(&records.get_untracked(), &id)
            .map(|r| r.cpm)
            .unwrap_or(0.0);
        persist_records(storage::push(
            &records.get_untracked(),
            Record {
                lesson_id: id,
                title: title.clone(),
                at_ms: now_ms,
                cpm: stats.cpm,
                accuracy: stats.accuracy,
                errors: stats.errors,
                chars: stats.correct,
                secs: stats.secs,
                errors_by_key: record_errors,
            },
        ));
        route.set(Route::Result(Finished {
            title,
            lang,
            stats,
            top_errors,
            best_cpm: previous_best.max(stats.cpm),
            is_record: stats.cpm > previous_best,
            shuangpin: shuangpin_mode.get_untracked(),
            sp: is_sp(),
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
        // 火力随时间衰减（衰减速率随蓄力上升），即时速度窗口同时收缩
        let profile = heat_profile();
        heat.update(|h| *h = profile.decay(*h, 0.1));
        if heat.with_untracked(|h| *h < heat::BURST_EXIT) {
            burst.set(false);
        }
        speed.update(|w| w.prune(now_ms));
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
        let before = session.with_untracked(|s| {
            let stats = s.stats(now_ms);
            (stats.cursor, stats.errors)
        });
        session.update(|s| s.sync_text(&text, now_ms));
        let after = session.with_untracked(|s| {
            let stats = s.stats(now_ms);
            (stats.cursor, stats.errors)
        });
        // 输入法一次可能提交多个字，按增量蓄力
        add_heat(
            after.0.saturating_sub(before.0),
            after.1.saturating_sub(before.1) as usize,
            now_ms,
        );
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
                let Some(buffer) = result.dyn_into::<js_sys::ArrayBuffer>().ok() else {
                    return;
                };
                let Some(text) = decode_text(&buffer) else {
                    return;
                };
                if text.trim().is_empty() {
                    return;
                }
                // 按内容归类，而不是按当前 tab：否则会出现「中文课里挂英文素材」这种打不出来的组合
                let lang = detect_lang(&text);
                persist_materials(materials::upsert(
                    &user_materials.get_untracked(),
                    Material {
                        id: user_material_id(),
                        name: name.clone(),
                        lang,
                        text,
                        bundled: false,
                    },
                ));
                // 切到对应语言，让导入结果立刻可见
                persist_tab(lang);
            });
            reader.set_onloadend(Some(on_done.as_ref().unchecked_ref()));
            // 一次性回调：导入是低频操作，读完就丢给浏览器回收
            on_done.forget();
            let _ = reader.read_as_array_buffer(&file);
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
        // 素材页选素材：中文素材按最近一次的中文模式分流到输入法 / 双拼键位
        Action::PracticeMaterial(id) => {
            let zh_shuangpin = all_materials
                .with_untracked(|all| all.iter().any(|m| m.id == id && m.lang == Lang::Zh))
                && zh_mode.get_untracked() == ZhMode::Shuangpin;
            if zh_shuangpin {
                start_shuangpin(None, Some(id));
            } else {
                start_material(id);
            }
        }
        Action::Test(secs) => start_segment(tab.get_untracked(), false, Some(secs)),
        Action::Shuangpin { secs } => start_shuangpin(secs, None),
        Action::Import => {
            if let Some(input) = file_ref.get() {
                input.click();
            }
        }
        Action::DeleteMaterial(id) => {
            persist_materials(materials::remove(&user_materials.get_untracked(), &id))
        }
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
                persist_materials(materials::upsert(
                    &user_materials.get_untracked(),
                    Material {
                        id: user_material_id(),
                        name,
                        lang,
                        text,
                        bundled: false,
                    },
                ));
                persist_tab(lang);
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
            Source::Shuangpin { secs, material_id } => start_shuangpin(secs, material_id),
        },
        Action::Back => back(),
    };

    // ---------- 菜单项（渲染与键盘共用同一份） ----------
    let items = Memo::new(move |_| {
        let lang = tab.get();
        let records_snapshot = records.get();
        // 中文模式为双拼键位时，素材项提示也跟着变
        let sp_mode = lang == Lang::Zh && zh_mode.get() == ZhMode::Shuangpin;
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
                        "双拼练习（跟自己的输入法）",
                        "输入法切到小鹤方案，配键位表练习",
                        Action::Segment { shuangpin: true },
                    ));
                    items.push(item(
                        "双拼键位（按键判定）",
                        "按小鹤码逐键判定，多音字接受任一读音，打错自动纠正",
                        Action::Shuangpin { secs: None },
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
            Route::Tests => {
                let mut tests = vec![
                    item("1 分钟测试", "计 1 分钟，到点结算", Action::Test(60)),
                    item("2 分钟测试", "计 2 分钟，到点结算", Action::Test(120)),
                    item("5 分钟测试", "计 5 分钟，到点结算", Action::Test(300)),
                ];
                if lang == Lang::Zh {
                    tests.push(item(
                        "双拼键位 1 分钟",
                        "小鹤码按键判定，计 1 分钟",
                        Action::Shuangpin { secs: Some(60) },
                    ));
                    tests.push(item(
                        "双拼键位 3 分钟",
                        "小鹤码按键判定，计 3 分钟",
                        Action::Shuangpin { secs: Some(180) },
                    ));
                }
                tests
            }
            Route::Material => {
                let mut items = vec![
                    item(
                        "导入 .txt 文件",
                        "可多选，按内容归入中文/英文；同名会覆盖",
                        Action::Import,
                    ),
                    item(
                        "粘贴文本新建",
                        "手动写名字和正文，同名会覆盖",
                        Action::Go(Route::NewMaterial),
                    ),
                ];
                let materials_snapshot = all_materials.get();
                for material in materials_snapshot.iter().filter(|m| m.lang == lang) {
                    let mode_hint = if sp_mode { "双拼键位" } else { "练习" };
                    let mut entry = item(
                        material.name.clone(),
                        format!(
                            "{} 字 · 回车{}",
                            material.len_chars(),
                            if material.bundled {
                                format!("{mode_hint}（随包素材）")
                            } else {
                                format!("{mode_hint}，Delete 删除")
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

    // 列表变化（删素材、切语言）后把高亮游标收回范围内
    Effect::new(move |_| {
        let len = items.get().len();
        cursor.update(|c| {
            if *c >= len {
                *c = len.saturating_sub(1);
            }
        });
    });

    // ---------- 键盘 ----------
    let _listener = StoredValue::new_local(KeyListener::new(move |ev: web_sys::KeyboardEvent| {
        if ev.ctrl_key() || ev.meta_key() || ev.alt_key() {
            return;
        }
        let key = ev.key();

        if route.get_untracked() == Route::Practice {
            match key.as_str() {
                // 输入法组词中 Esc 是「取消候选」，不能顺手退出练习
                "Escape" => {
                    if ev.is_composing() {
                        return;
                    }
                    back()
                }
                "Backspace" => {
                    if !uses_input_field() {
                        ev.prevent_default();
                        session.update(|s| s.backspace());
                    }
                }
                _ => {
                    // 中文输入法的字符交给输入框，不在这里判定
                    if uses_input_field() {
                        return;
                    }
                    // 输入法正在组词（或系统把按键标成 Process）：不该计入英文练习
                    if ev.is_composing() || key == "Process" {
                        ime_notice.set(true);
                        return;
                    }
                    let mut chars = key.chars();
                    if let (Some(ch), None) = (chars.next(), chars.next()) {
                        let ch = if is_sp() {
                            // 双拼只判定字母键；空格/数字/标点不参与
                            let lowered = ch.to_ascii_lowercase();
                            if !lowered.is_ascii_alphabetic() {
                                return;
                            }
                            lowered
                        } else {
                            if !(ch.is_ascii_graphic() || ch == ' ') {
                                return;
                            }
                            ch
                        };
                        if session.with_untracked(|s| s.is_empty() || s.finished()) {
                            return;
                        }
                        ev.prevent_default();
                        let now_ms = js_sys::Date::now();
                        let mut result = KeyResult::Ignored;
                        session.update(|s| result = s.press_char(ch, now_ms));
                        match result {
                            KeyResult::Correct => add_heat(1, 0, now_ms),
                            KeyResult::Mistake => add_heat(0, 1, now_ms),
                            KeyResult::Ignored => {}
                        }
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
                persist_tab(next);
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
            // 素材页：M 切换中文练习模式（输入法 / 双拼键位）
            "m" | "M" => {
                if route.get_untracked() == Route::Material && tab.get_untracked() == Lang::Zh {
                    ev.prevent_default();
                    zh_mode.update(|mode| {
                        *mode = match mode {
                            ZhMode::Ime => ZhMode::Shuangpin,
                            ZhMode::Shuangpin => ZhMode::Ime,
                        };
                    });
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
        // 双拼按键模式：给出当前字的拼音与常用编码
        if let Some((ch, pinyin, code)) = session.with(|s| s.sp_current()) {
            return format!("下一个：{ch} · 拼音 {pinyin} · 小鹤 {}{}", code[0], code[1]);
        }
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

    // 双拼按键模式的目标格：字 + 拼音 + 常用编码
    let sp_cells = move || {
        session.with(|s| {
            let Some(targets) = s.sp_targets() else {
                return ().into_any();
            };
            let cursor = s.cursor();
            let mistake = s.mistake_at();
            targets
                .iter()
                .enumerate()
                .map(|(i, target)| {
                    let mut class = String::from("sp-cell");
                    match s.state_at(i) {
                        CharState::Todo => {}
                        CharState::Done => class.push_str(" done"),
                        CharState::Wrong => class.push_str(" wrong"),
                    }
                    if mistake == Some(i) {
                        class.push_str(" flash");
                    } else if i == cursor {
                        class.push_str(" current");
                    }
                    view! {
                        <span class=class>
                            <b>{target.ch.to_string()}</b>
                            <i>{target.pinyin.to_string()}</i>
                            <em>{format!("{}{}", target.codes[0][0], target.codes[0][1])}</em>
                        </span>
                    }
                })
                .collect_view()
                .into_any()
        })
    };

    // 练习文本自动跟随游标：限时测试会不断续段，不跟随的话游标很快滚出视口
    Effect::new(move |_| {
        let _ = session.with(|s| s.cursor());
        let Some(container) = text_ref.get() else {
            return;
        };
        let Some(cur) = container
            .query_selector(".ch.cursor, .sp-cell.current")
            .ok()
            .flatten()
        else {
            return;
        };
        let options = web_sys::ScrollIntoViewOptions::new();
        options.set_block(web_sys::ScrollLogicalPosition::Nearest);
        cur.scroll_into_view_with_scroll_into_view_options(&options);
    });

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
            <h1>"打字训练"</h1>
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
                                                    persist_tab(l);
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

        {move || {
            storage_error
                .get()
                .map(|message| {
                    view! {
                        <div class="warn storage-warn">
                            <span>{message}</span>
                            <button class="btn ghost" on:click=move |_| storage_error.set(None)>
                                "知道了"
                            </button>
                        </div>
                    }
                })
        }}

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
                                        {move || {
                                            (tab.get() == Lang::Zh)
                                                .then(|| {
                                                    view! {
                                                        <div class="mode-row">
                                                            <span class="k">"练习模式"</span>
                                                            <div class="tabs">
                                                                <button
                                                                    class=move || if zh_mode.get() == ZhMode::Ime { "tab on" } else { "tab" }
                                                                    on:click=move |_| zh_mode.set(ZhMode::Ime)
                                                                >
                                                                    "输入法判定"
                                                                </button>
                                                                <button
                                                                    class=move || if zh_mode.get() == ZhMode::Shuangpin { "tab on" } else { "tab" }
                                                                    on:click=move |_| zh_mode.set(ZhMode::Shuangpin)
                                                                >
                                                                    "双拼键位"
                                                                </button>
                                                            </div>
                                                            <span class="mode-hint">
                                                                "按 M 切换 · 点素材按当前模式开练"
                                                            </span>
                                                        </div>
                                                    }
                                                })
                                        }}
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
                    history_view(records, back, move || persist_records(Vec::new())).into_any()
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
                                        {move || if uses_input_field() { "错字" } else { "错误" }}
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

                            {move || {
                                let h = heat.get();
                                let profile = heat_profile();
                                let power = profile.burst_power(speed.with(|w| w.cpm()));
                                let full = burst.get();
                                let class = if full { "heat full" } else { "heat" };
                                // 蓄力越高越鲜艳；爆发强度（速度）控制辉光、火花距离与动画快慢
                                let style = format!(
                                    "--sat:{:.0}%;--bri:{:.0}%;--glow:{:.1}px;--spark:{:.2};--dur:{:.2}s",
                                    60.0 + h * 140.0,
                                    72.0 + h * 55.0,
                                    5.0 + power * 15.0,
                                    0.55 + power * 1.05,
                                    0.8 - power * 0.4,
                                );
                                let fill = format!(
                                    "width:{:.1}%;background-position-x:{:.0}%",
                                    h * 100.0,
                                    h * 100.0,
                                );
                                // 没标签了，用悬浮提示说明玩法与「压满线」
                                let unit = if is_zh() && !is_sp() { "字" } else { "键" };
                                let fill_cpm = profile.decay_max / profile.gain * 60.0;
                                let title = format!(
                                    "打字蓄力：满格爆发，速度越快特效越强（约 {fill_cpm:.0} {unit}/分可压满）",
                                );
                                view! {
                                    <div class=class style=style title=title>
                                            <i style=fill></i>
                                            {full
                                                .then(|| {
                                                    (0..6)
                                                        .map(|i| {
                                                            view! { <b class=format!("spark s{i}")></b> }
                                                        })
                                                        .collect_view()
                                                })}
                                        </div>
                                }
                            }}

                            <div
                                node_ref=text_ref
                                class=move || match (is_zh(), is_sp()) {
                                    (_, true) => "text zh sp",
                                    (true, false) => "text zh",
                                    _ => "text",
                                }
                            >
                                {move || if is_sp() { sp_cells() } else { text_spans().into_any() }}
                            </div>

                            <div class="hint">{hint}</div>
                            {move || {
                                ime_notice
                                    .get()
                                    .then(|| {
                                        view! {
                                            <p class="warn">
                                                {if is_sp() {
                                                    "检测到输入法正在组词：双拼键位练习需要英文键盘状态（小鹤码是字母），请先切回英文键盘。"
                                                } else {
                                                    "检测到输入法正在组词：英文练习请先切回英文键盘（macOS 可用 Caps Lock 或 ⌃Space），组词中的击键不会被计入。"
                                                }}
                                            </p>
                                        }
                                    })
                            }}

                            {move || if uses_input_field() {
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
                    let sp = finished.sp;
                    let cells: Vec<(&'static str, String)> = if zh {
                        vec![
                            ("正确率", format!("{:.1}%", stats.accuracy)),
                            (
                                if sp { "击键 / 错误" } else { "输入 / 错字" },
                                format!("{} / {}", stats.strokes, stats.errors),
                            ),
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
                    let problem_label = if zh && !sp { "错字" } else { "问题键" };
                    let title = finished.title.clone();
                    let mode_note = if sp {
                        "双拼键位 · 按小鹤码逐键判定，多音字接受任一读音"
                    } else if finished.shuangpin {
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
