use prelude::*;

/// Prelude module. Used to re-export commonly used items.
pub mod prelude {
	pub use leptos::*;

	pub use crate::{components::*, utils::*};
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
	view! {
		cx,
		<div class="app">
			<Icon icon="test" />
		</div>
	}
}
