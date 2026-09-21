mod app;
mod bundled;
mod cn_engine;
mod dom;
mod engine;
mod history;
mod keyboard;
mod layout;
mod lessons;
mod materials;
mod model;
mod pinyin;
mod progress;
mod rng;
mod segment;
mod session;
mod settings;
mod shuangpin;
mod sp_engine;
mod storage;
mod store;

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
