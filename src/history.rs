//! 成绩档案：每课的练习次数、最佳/最近成绩与速度趋势。

use leptos::prelude::*;

use crate::lessons::LESSONS;
use crate::progress::spark_points;
use crate::storage::{self, Record};

pub fn history_view(
    records: RwSignal<Vec<Record>>,
    on_back: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let confirming = RwSignal::new(false);

    let summary = move || {
        let all = records.get();
        let sessions = all.len();
        let secs: f64 = all.iter().map(|r| r.secs).sum();
        let chars: usize = all.iter().map(|r| r.chars).sum();
        let accuracy = if sessions == 0 {
            0.0
        } else {
            all.iter().map(|r| r.accuracy).sum::<f64>() / sessions as f64
        };
        format!(
            "共 {sessions} 次练习 · 累计 {} · 打对 {chars} 字 · 平均正确率 {accuracy:.0}%",
            fmt_duration(secs)
        )
    };

    let rows = move || {
        let all = records.get();
        // 按出现顺序列出练过的课程（每次练习都是追加，天然按时间排列）
        let mut ids: Vec<String> = Vec::new();
        for record in &all {
            if !ids.iter().any(|id| id == &record.lesson_id) {
                ids.push(record.lesson_id.clone());
            }
        }
        ids.into_iter()
            .map(|id| {
                let history: Vec<Record> =
                    all.iter().filter(|r| r.lesson_id == id).cloned().collect();
                let stored_title = history
                    .last()
                    .map(|r| r.title.clone())
                    .filter(|title| !title.is_empty());
                let title = stored_title
                    .or_else(|| {
                        LESSONS
                            .iter()
                            .find(|l| l.id == id)
                            .map(|l| l.title.to_string())
                    })
                    .unwrap_or_else(|| id.clone());
                let best = history.iter().map(|r| r.cpm).fold(0.0, f64::max);
                let recent = history.last().map(|r| r.cpm).unwrap_or(0.0);
                let accuracy =
                    history.iter().map(|r| r.accuracy).sum::<f64>() / history.len() as f64;
                let values: Vec<f64> = history.iter().rev().take(12).rev().map(|r| r.cpm).collect();

                view! {
                    <div class="history-row">
                        <div class="history-main">
                            <span class="history-title">{title}</span>
                            <span class="history-meta">
                                {format!(
                                    "练习 {} 次 · 平均正确率 {:.0}%",
                                    history.len(),
                                    accuracy,
                                )}
                            </span>
                        </div>
                        <div class="history-score">
                            <span class="history-recent">{format!("最近 {recent:.0}")}</span>
                            <span class="history-best">{format!("最佳 {best:.0}")}</span>
                        </div>
                        {match spark_points(&values, 100.0, 30.0) {
                            Some(points) => {
                                view! {
                                    <svg class="spark" viewBox="0 0 100 30">
                                        <polyline
                                            points=points
                                            style="fill:none;stroke:currentColor;stroke-width:1.5"
                                        ></polyline>
                                    </svg>
                                }
                                    .into_any()
                            }
                            None => view! { <svg class="spark empty" viewBox="0 0 100 30"></svg> }
                                .into_any(),
                        }}
                    </div>
                }
            })
            .collect_view()
    };

    view! {
        <section class="history">
            <div class="toolbar">
                <button class="btn ghost" on:click=move |_| on_back()>
                    "返回课程表"
                </button>
                {move || {
                    if records.get().is_empty() {
                        view! { <span></span> }.into_any()
                    } else if confirming.get() {
                        view! {
                            <span class="label">"确定清空全部成绩？"</span>
                            <button
                                class="btn danger"
                                on:click=move |_| {
                                    storage::clear();
                                    records.set(Vec::new());
                                    confirming.set(false);
                                }
                            >
                                "确定清空"
                            </button>
                            <button class="btn ghost" on:click=move |_| confirming.set(false)>
                                "取消"
                            </button>
                        }
                            .into_any()
                    } else {
                        view! {
                            <button class="btn ghost" on:click=move |_| confirming.set(true)>
                                "清空成绩"
                            </button>
                        }
                            .into_any()
                    }
                }}
            </div>

            <p class="lede">{summary}</p>

            {move || {
                if records.get().is_empty() {
                    view! { <p class="empty">"还没有成绩记录，先练一课吧。"</p> }.into_any()
                } else {
                    view! { <div class="history-list">{rows()}</div> }.into_any()
                }
            }}
        </section>
    }
}

fn fmt_duration(secs: f64) -> String {
    if secs < 60.0 {
        format!("{secs:.0} 秒")
    } else if secs < 3600.0 {
        format!("{:.0} 分钟", secs / 60.0)
    } else {
        format!("{:.1} 小时", secs / 3600.0)
    }
}
