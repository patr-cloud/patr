use crate::prelude::*;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
	view! { cx,
		<Login />
	}
}
