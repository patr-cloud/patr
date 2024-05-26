use crate::imports::*;

type Gap = Size;

#[component]
pub fn DashboardContainer(
	/// Additional classes to apply to the outer section
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// All the items to be rendered, does not iterate,
	/// send the <For /> component or all the rows in the component.
	render_items: View,
	/// Gap between each item in Dashboard grid
	#[prop(optional)]
	gap: Gap,
	/// Alignment of the grid items
	#[prop(optional)]
	_align: Alignment,
) -> impl IntoView {
	let class = move || format!("p-xl full-width ofy-auto {}", class.get());
	let div_class = move || format!("grid-cnt-st-st grid-col-3 gap-{}", gap.as_css_name());

	view! {
		<section class={class}>
			<div class={div_class}>{render_items}</div>
		</section>
	}
}
