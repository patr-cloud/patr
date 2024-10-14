use crate::imports::*;

/// The Main Container for all the content, typically used alongside the sidebar
/// and used for LoggedIn Routes
#[component]
pub fn ContainerMain(
	/// Additional class names to apply to the outer header, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The Children of the component
	children: Children,
) -> impl IntoView {
	let class = move || {
		format!(
            "flex flex-col items-start justify-start bg-secondary-dark w-full h-full mb-md br-sm overflow-hidden {}",
            class.get()
        )
	};

	view! { <section class={class}>{children()}</section> }
}
