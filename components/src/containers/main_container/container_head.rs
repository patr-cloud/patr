use crate::imports::*;

/// Contains the title, description and the DocLink of the Page,
/// Usually wrapping around the <PageTitle /> section of components
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
			"flex flex-col items-start justify-start px-xl py-md bg-secondary-light w-full {}",
			class.get()
		)
	};

	view! { <header class={class}>{children()}</header> }
}
