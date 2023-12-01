use crate::prelude::*;

#[derive(Default)]
pub struct LinkItem {
	pub title: String,
	pub path: AppRoute,
	pub items: Option<Vec<LinkItem>>,
}

// Sidebar component
#[component]
pub fn Sidebar() -> impl IntoView {
	let LINKS: Vec<LinkItem> = vec![
		LinkItem {
			title: "Home".to_string(),
			path: AppRoute::LoggedInRoute(LoggedInRoute::Home),
			items: None,
		},
		LinkItem {
			title: "BYOC".to_string(),
			path: AppRoute::LoggedInRoute(LoggedInRoute::Home),
			items: None,
		},
	];

	view! {
		<div class="sidebar fc-fs-fs">
			<div class="sidebar-logo">
				<img href="/images/planet-purple.svg" alt="Planet Patr" />
				<div class="fc-ct-ct br-sm">
					<img href="/images/patr.svg" alt="Patr Logo" />
				</div>
			</div>

			<div class="full-width full-height fc-fs-fs of-hidden pt-md">
				<nav class="full-width full-height fc-fs-fs">
					{
						LINKS
							.into_iter()
							.map(|link| view! {
                                // <ALink to={link.path}>
                                //     {link.title}
                                // </ALink>
                                <SidebarItem link={link} />
							})
							.collect_view()
					}
				</nav>
			</div>
		</div>
	}
}
