mod app;
mod constants;
#[allow(dead_code)]
mod pages;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run_app() -> Result<(), JsValue> {
	yew::start_app::<app::App>();

	Ok(())
}
