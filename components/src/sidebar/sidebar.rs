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
pub fn Sidebar() -> impl IntoView {
	let links: Vec<LinkItem> = vec![
		LinkItem {
			title: "Home".to_owned(),
			path: "/".to_owned(),
			icon_src: "/images/sidebar/home.svg".to_owned(),
			subtitle: None,
			items: None,
		},
		LinkItem {
			title: "BYOC".to_owned(),
			path: "/".to_owned(),
			icon_src: "/images/sidebar/byoc.svg".to_owned(),
			subtitle: Some("Connect your cloud".to_owned()),
			items: None,
		},
		LinkItem {
			title: "Deployments".to_owned(),
			path: "deployment".to_owned(),
			icon_src: "/images/sidebar/infrastructure.svg".to_owned(),
			subtitle: None,
			items: None,
		},
		LinkItem {
			title: "Container Registry".to_owned(),
			path: "container-registry".to_owned(),
			icon_src: "/images/sidebar/docker.svg".to_owned(),
			subtitle: None,
			items: Some(vec![
				LinkItem {
					title: "Domains".to_owned(),
					path: "/".to_owned(),
					subtitle: None,
					icon_src: "".to_owned(),
					items: None,
				},
				LinkItem {
					title: "Managed URLs".to_owned(),
					icon_src: "".to_owned(),
					subtitle: None,
					path: "/".to_owned(),
					items: None,
				},
			]),
		},
		LinkItem {
			title: "Profile".to_owned(),
			path: "profile".to_owned(),
			icon_src: "/images/sidebar/workspace.svg".to_owned(),
			subtitle: None,
			items: None,
		},
	];

	view! {
		<aside class="sidebar fc-fs-fs">
			<div class="sidebar-logo">
				<img src="/images/planet-purple.svg" alt="Plante Patr"/>
				<div class="fc-ct-ct br-sm">
					<img src="/images/patr.svg" alt="Patr Logo"/>
				</div>
			</div>

			<div class="full-width full-height fc-fs-fs of-hidden pt-md">
				<nav class="full-height full-width fc-fs-fs ofy-auto mt-md">
					<ul class="full-width full-height fc-fs-fs">

						{links
							.into_iter()
							.map(|link| view! { <SidebarItem link={link}/> })
							.collect_view()}

					</ul>
				</nav>
			</div>
		</aside>
	}
}
