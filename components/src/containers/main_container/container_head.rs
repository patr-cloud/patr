use crate::imports::*;

#[component]
pub fn ContainerHead(
	/// Additional class names to apply to the outer header, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The Children of the component
	children: Children,
) -> impl IntoView {
	let class = move || {
		format!(
			"fc-fs-fs px-xl py-md bg-secondary-light full-width {}",
			class.get()
		)
	};

	view! {
		<header class=class>
			{children()}
		</header>
	}
}
