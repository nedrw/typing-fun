//! 少量 DOM 胶水：定时器与全局键盘监听。
//!
//! 直接用 web-sys 写，省掉一层封装；两者都在 drop 时自己清理。

use wasm_bindgen::prelude::*;

/// 周期定时器，drop 时自动清除。
pub struct Ticker {
    id: i32,
    _cb: Closure<dyn Fn()>,
}

impl Ticker {
    pub fn start(period_ms: i32, cb: impl Fn() + 'static) -> Option<Self> {
        let cb: Closure<dyn Fn()> = Closure::new(cb);
        let id = web_sys::window()?
            .set_interval_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                period_ms,
            )
            .ok()?;
        Some(Self { id, _cb: cb })
    }
}

impl Drop for Ticker {
    fn drop(&mut self) {
        if let Some(window) = web_sys::window() {
            window.clear_interval_with_handle(self.id);
        }
    }
}

/// window 上的键盘监听，drop 时自动移除。
pub struct KeyListener {
    target: web_sys::EventTarget,
    cb: Closure<dyn FnMut(web_sys::KeyboardEvent)>,
}

impl KeyListener {
    pub fn new(cb: impl FnMut(web_sys::KeyboardEvent) + 'static) -> Option<Self> {
        let window = web_sys::window()?;
        let target: web_sys::EventTarget = window.unchecked_into();
        let cb: Closure<dyn FnMut(web_sys::KeyboardEvent)> = Closure::new(cb);
        target
            .add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref())
            .ok()?;
        Some(Self { target, cb })
    }
}

impl Drop for KeyListener {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback("keydown", self.cb.as_ref().unchecked_ref());
    }
}

/// window 上的输入法组词监听（`compositionstart`），drop 时自动移除。
///
/// 英文练习里组词意味着用户在中文输入法下敲键，这些击键不该计入统计。
pub struct CompositionListener {
    target: web_sys::EventTarget,
    cb: Closure<dyn FnMut(web_sys::CompositionEvent)>,
}

impl CompositionListener {
    pub fn new(cb: impl FnMut(web_sys::CompositionEvent) + 'static) -> Option<Self> {
        let window = web_sys::window()?;
        let target: web_sys::EventTarget = window.unchecked_into();
        let cb: Closure<dyn FnMut(web_sys::CompositionEvent)> = Closure::new(cb);
        target
            .add_event_listener_with_callback("compositionstart", cb.as_ref().unchecked_ref())
            .ok()?;
        Some(Self { target, cb })
    }
}

impl Drop for CompositionListener {
    fn drop(&mut self) {
        let _ = self.target.remove_event_listener_with_callback(
            "compositionstart",
            self.cb.as_ref().unchecked_ref(),
        );
    }
}
