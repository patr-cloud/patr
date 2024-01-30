use crate::imports::*;

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
            "fc-fs-fs bg-secondary-dark full-width full-height mb-md box-full-main br-sm of-hidden {}",
            class.get()
        )
	};

	view! {
		<section class=class>
			{children()}
		</section>
	}
}
