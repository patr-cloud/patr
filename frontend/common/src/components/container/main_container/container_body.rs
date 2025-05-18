use crate::prelude::*;

/// The Body of the dashboard. Wraps around the main content of the page.
#[component]
pub fn ContainerBody(
	/// The Children of the component
	children: Children,
	/// Additional Class Names to be given to the outer div
	#[prop(into, optional)]
	class: Signal<String>,
) -> Element {
	let class = move || {
		format!(
			"relative flex flex-col items-center justify-start w-full h-full overflow-y-auto container-body {}",
			class.get()
		)
	};

	view! {
		<div class={class}>
			{children()}
		</div>
	}
}
