use crate::imports::*;

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
	gap: Size,
) -> impl IntoView {
	let class = move || format!("p-xl w-full overflow-y-auto {}", class.get());

	let div_class = move || {
		format!(
			"grid content-start justify-start grid-cols-3 gap-{}",
			gap.as_css_name()
		)
	};

	view! {
		<section class={class}>
			<div class={div_class}>{render_items}</div>
		</section>
	}
}
