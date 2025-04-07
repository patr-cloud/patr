use std::rc::Rc;

use leptos_router::{use_location, use_navigate};

use crate::imports::*;

#[component]
pub fn SidebarExpanded(
	/// Expanded Link Items
	#[prop(into)]
	link_items: Signal<Vec<LinkItem>>,
) -> impl IntoView {
	view! {
		<ul class="w-full h-full flex flex-col items-start justify-start">
			{link_items
				.get()
				.into_iter()
				.map(|link| view! {
					<SidebarItem
						is_nested={true}
						class="bg-secondary-dark border-border-color border-b"
						link={link}
					/>
				})
				.collect_view()}
		</ul>
	}
}

#[component]
pub fn SidebarItem(
	/// Link info of the link item
	#[prop(into)]
	link: MaybeSignal<LinkItem>,
	/// Additional classes to add to the sidebar if necessary
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Whether is the Sidebar Item is nested or root
	#[prop(into, optional, default = false.into())]
	is_nested: MaybeSignal<bool>,
) -> impl IntoView {
	let store_link = store_value(link.clone());
	let is_expanded = create_rw_signal(false);
	let navigate = use_navigate();
	let location = use_location();

	let class = move || {
		format!(
			"sidebar-item w-full flex flex-col items-start justify-start {} {} {} {}",
			class.get(),
			if is_nested.get() {
				"inner-nav-item"
			} else {
				""
			},
			if store_link.with_value(|val| val.get().items.is_some()) && is_expanded.get() {
				"bg-secondary-light text-white/100"
			} else {
				""
			},
			if store_link
				.with_value(|link| location.pathname.get().contains(link.get().path.as_str()))
			{
				"active-nav-item"
			} else {
				""
			}
		)
	};

	let items = move || {
		store_link.with_value(|link| {
			link.get().items.map(|items| {
				view! {
					<SidebarExpanded link_items={Signal::derive(move || items.clone())}/>
				}
			})
		})
	};

	view! {
		<li class={class}>
			<Link
				r#type={Variant::Button}
				style_variant={LinkStyleVariant::Plain}
				class={"btn w-full py-sm"}
				on_click={
					let navigate = navigate.clone();
					Rc::new(move |_| {
						is_expanded.update(|val| *val = !*val);
						navigate(
							store_link.with_value(|link| link.get().path).as_str(),
							Default::default()
						);
					})
				}
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

			<Show
				when={move || is_expanded.get()}
			>
				{items}
			</Show>
		</li>
	}
}
