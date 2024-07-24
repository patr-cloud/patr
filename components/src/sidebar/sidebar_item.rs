use crate::imports::*;

#[component]
pub fn SidebarItem(
	/// Link info of the link item
	#[prop(into)]
	link: MaybeSignal<LinkItem>,
	/// Additional classes to add to the sidebar if necessary
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let class = move || {
		format!(
			"sidebar-item w-full flex items-start justify-start {}",
			class.get()
		)
	};

	let store_link = store_value(link.clone());

	view! {
		<li class={class}>
			<Link
				to={store_link.with_value(|link| link.get().path)}
				r#type={Variant::Link}
				style_variant={LinkStyleVariant::Plain}
				class="btn w-full py-sm"
			>
				<img src={link.get().icon_src} alt={link.get().title}/>
				<span class="ml-md text-md flex items-start justify-start txt-left">
					<span class="pos-rel text-md txt-left">
						{move || store_link.with_value(|link| link.get().title)}
					</span>
					<small class="text-xxs txt-grey">
						{move || store_link.with_value(|link| link.get().subtitle)}
					</small>
				</span>
			</Link>
		</li>
	}
}
