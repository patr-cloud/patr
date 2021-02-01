#![recursion_limit = "1024"]

mod models;
mod ui;

use ui::App;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
	#[wasm_bindgen(js_namespace = window)]
	fn initMaterialize();
}

pub fn init_materialize() {
	initMaterialize();
}

fn main() {
	yew::start_app::<App>();
}
