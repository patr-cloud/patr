use strum::Display;

use crate::imports::*;

/// The AutoSizing of the Grid Item
#[derive(Display, Clone, Copy)]
pub enum AutoSizing {
	#[strum(to_string = "auto-fill")]
	/// The Items will NOT take up the whole space, new columns will be added
	Fill,
	/// The Items will take up the whole space.
	#[strum(to_string = "auto-fit")]
	Fit,
}

/// The Body of the dashboard. Wraps around the main content of the page.
#[component]
pub fn ContainerGrid(
	/// The Children of the component
	children: Children,
	/// Additional Classnames to be given to the outer div
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Minimum Width of the Grid Item, Defaults to 300px.
	#[prop(into, optional, default = "300px".into())]
	min_width: MaybeSignal<String>,
	/// Minimum Width of the Grid Item, Defaults to 1fr.
	#[prop(into, optional, default = "1fr".into())]
	max_width: MaybeSignal<String>,
	/// The Fit of the Grid Item, Defaults to Fill.
	#[prop(into, optional, default = AutoSizing::Fill)]
	auto_sizing: AutoSizing,
) -> impl IntoView {
	let class = move || format!("grid gap-lg justify-start content-start {}", class.get());

	let style = move || {
		format!(
			"grid-template-columns: repeat({}, minmax({}, {}));",
			auto_sizing.to_string(),
			min_width.get(),
			max_width.get()
		)
	};

	view! {
		<section
			class="p-xl w-full overflow-y-auto"
		>
			<div
				style={style}
				class={class}
			>
				{children()}
			</div>
		</section>
	}
}
