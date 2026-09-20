mod app;
mod cn_engine;
mod dom;
mod engine;
mod history;
mod keyboard;
mod layout;
mod lessons;
mod manifest;
mod materials;
mod model;
mod progress;
mod rng;
mod segment;
mod session;
mod shuangpin;
mod storage;

use app::*;
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! {
            <App/>
        }
    })
}
