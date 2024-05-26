use crate::prelude::*;

/// A badge. Used for things like unread counts, "beta" labels, etc.
#[component]
pub fn Badge(
	/// Any additional classes to apply to the badge.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The color of the badge.
	#[prop(into, optional)]
	color: MaybeSignal<Color>,
	/// The text in the badge.
	#[prop(into, optional)]
	text: MaybeSignal<String>,
) -> impl IntoView {
	view! {
		<span class={move || {
			format!(
				"badge pos-abs txt-secondary txt-medium bg-{} {}",
				color.get().as_css_name(),
				class.get(),
			)
		}}>{text}</span>
	}
}
