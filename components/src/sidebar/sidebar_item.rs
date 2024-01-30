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
	let class = move || format!("sidebar-item full-width fc-fs-fs {}", class.get());

	view! {
		/// Will add the class later
		<li class=class>
			<Link style_variant=LinkStyleVariant::Plain class="btn full-width py-sm">
				<img src={link.get().icon_src} alt={link.get().title} />
				<span class="ml-md txt-md fc-fs-fs txt-left">
					<span class="pos-rel txt-md txt-left">
						{link.get().title}
					</span>
					<small class="txt-xxs txt-grey">
						{link.get().subtitle}
					</small>
				</span>
			</Link>
		</li>
	}
}
