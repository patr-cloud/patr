use crate::prelude::*;

/// A Tab Item
#[derive(Clone, PartialEq, Eq)]
pub struct TabItem {
	/// Name of the tab to navigate to
	pub name: String,
	/// The Path to navigate to
	pub path: String,
}

/// Various Tabs to navigate to different pages in the same group
#[component]
pub fn Tabs(
	/// Additional class names to apply to the external div
	#[prop(into, optional)]
	class: Signal<String>,
	/// The Tab Item
	#[prop(into, optional)]
	tab_items: Signal<Vec<TabItem>>,
) -> Element {
	let class = class.with(|cname| format!("flex justify-start items-end {cname}"));

	view! {
		<div class={class}>
			{tab_items
				.get()
				.into_iter()
				.map(|n| {
					view! {
						<Link
							to={n.clone().path}
							class="tab-item mx-xl"
							variant={LinkStyleVariant::Plain}
							color={Color::Grey}
						>
							{n.clone().name}
						</Link>
					}
				})
				.collect_view()
			}
		</div>
	}
}
