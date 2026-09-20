//! 虚拟键盘视图：标出下一个要按的键、该用哪根手指、要不要另一只手按 Shift。

use leptos::prelude::*;

use crate::layout::{self, finger_class, Hand, ROWS};

/// 画一块虚拟键盘。`next` 返回下一个待输入字符，用于高亮提示。
pub fn virtual_keyboard<F>(next: F) -> impl IntoView
where
    F: Fn() -> Option<char> + Copy + Send + Sync + 'static,
{
    view! {
        <div class="keyboard">
            {ROWS
                .iter()
                .enumerate()
                .map(|(row_index, row)| {
                    let keys = row
                        .chars()
                        .map(|ch| view! { <kbd class=key_class(next, ch)>{ch.to_string()}</kbd> })
                        .collect_view();
                    view! { <div class=format!("krow krow-{row_index}")>{keys}</div> }
                })
                .collect_view()}
            <div class="krow krow-4">
                <kbd class=move || shift_class(next, false)>"Shift"</kbd>
                <kbd class=move || space_class(next)>"空格"</kbd>
                <kbd class=move || shift_class(next, true)>"Shift"</kbd>
            </div>
        </div>
    }
}

fn key_class<F>(next: F, ch: char) -> impl Fn() -> String + Clone + Send + Sync + 'static
where
    F: Fn() -> Option<char> + Copy + Send + Sync + 'static,
{
    move || {
        let mut class = String::from("key");
        if let Some(finger) = layout::finger_of(ch) {
            class.push(' ');
            class.push_str(finger_class(finger));
        }
        if next().and_then(layout::base_key) == Some(ch) {
            class.push_str(" next");
        }
        class
    }
}

fn space_class<F>(next: F) -> String
where
    F: Fn() -> Option<char> + Copy + Send + Sync + 'static,
{
    let mut class = String::from("key space");
    if next() == Some(' ') {
        class.push_str(" next");
    }
    class
}

/// 该按哪个 Shift：与输入手指相反的那只手。
fn shift_class<F>(next: F, right: bool) -> String
where
    F: Fn() -> Option<char> + Copy + Send + Sync + 'static,
{
    let mut class = String::from("key wide");
    if let Some(ch) = next().filter(|c| layout::needs_shift(*c)) {
        let shift_hand = match layout::finger_of(ch).map(layout::hand) {
            Some(Hand::Left) => Hand::Right,
            _ => Hand::Left,
        };
        if (shift_hand == Hand::Right) == right {
            class.push_str(" next");
        }
    }
    class
}
