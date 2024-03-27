use crate::imports::*;

#[derive(Clone, PartialEq, Eq)]
pub struct TabItem {
	pub name: String,
	pub path: String,
}

#[component]
pub fn Tabs(
	/// Additional class names to apply to the external div
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The Tab Item
	#[prop(into, optional)]
	tab_items: MaybeSignal<Vec<TabItem>>,
) -> impl IntoView {
	let class = class.with(|cname| format!("fr-fs-fe {cname}"));

	view! {
		<div class=class>
			{
				tab_items.get().into_iter()
					.map(|n| view! {
						<Link
							r#type=Variant::Link
							to={n.clone().path}
							class="tab-item mx-xl"
							style_variant=LinkStyleVariant::Plain
							color=Color::Grey
						>
							{n.clone().name}
						</Link>
					})
					.collect_view()
			}
		</div>
	}
}
