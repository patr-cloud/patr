//! Main dashboard console for Patr

fn main() {
	dioxus::LaunchBuilder::web().launch(frontend::app::App);
}
