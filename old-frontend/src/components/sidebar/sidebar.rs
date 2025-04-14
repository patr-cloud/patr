use crate::imports::*;

/// A Link item for the sidebar
#[derive(Default, Clone)]
pub struct LinkItem {
	/// The Title of the link
	pub title: String,
	/// The Path to navigate to on clicking the link
	pub path: String,
	/// The Icon of the link
	pub icon_src: String,
	/// The Subtitle of the link, put any additional info or such in here
	pub subtitle: Option<String>,
	/// The Sub Items of the link, if any
	pub items: Option<Vec<LinkItem>>,
}

/// The Sidebar component containing the sidebar items
#[component]
pub fn Sidebar(
	/// Workspace Card
	children: ChildrenFn,
	/// The Sidebar Items
	sidebar_items: Vec<LinkItem>,
) -> impl IntoView {
	view! {
		<aside class="sidebar flex flex-col items-start justify-start pb-xl">
			<div class="sidebar-logo w-full">
				<img src="/images/planet-purple.svg" alt="Plante Patr"/>
				<div class="fc-ct-ct br-sm">
					<img src="/images/patr.svg" alt="Patr Logo"/>
				</div>
			</div>

			<div class="w-full h-full flex flex-col items-start justify-between overflow-hidden pt-md">
				<nav class="h-full w-full flex flex-col items-start justify-start overflow-y-auto mt-md">
					<ul class="w-full h-full flex flex-col items-start justify-start">
						{sidebar_items
							.into_iter()
							.map(|link| view! { <SidebarItem link={link}/> })
							.collect_view()}
					</ul>
				</nav>

				<div class="flex items-start justify-start w-full pt-md px-md">
					{children()}
				</div>
			</div>
		</aside>
	}
}
