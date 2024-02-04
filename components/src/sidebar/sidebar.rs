use crate::imports::*;

#[derive(Default, Clone)]
pub struct LinkItem {
	pub title: String,
	pub path: String,
	pub icon_src: String,
	pub subtitle: Option<String>,
	pub items: Option<Vec<LinkItem>>,
}

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
			title: "Infrastructure".to_owned(),
			path: "/".to_owned(),
			icon_src: "/images/sidebar/infrastructure.svg".to_owned(),
			subtitle: None,
			items: None,
		},
		LinkItem {
			title: "Domain Configuration".to_owned(),
			path: "/".to_owned(),
			icon_src: "/images/sidebar/domains.svg".to_owned(),
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
			title: "CI/CD".to_owned(),
			path: "/".to_owned(),
			icon_src: "/images/sidebar/cicd.svg".to_owned(),
			subtitle: None,
			items: None,
		},
		LinkItem {
			title: "Workspace".to_owned(),
			path: "/".to_owned(),
			icon_src: "/images/sidebar/deployment.svg".to_owned(),
			subtitle: None,
			items: None,
		},
	];

	view! {
		<aside class="sidebar fc-fs-fs">
			<div class="sidebar-logo">
				<img src="/images/planet-purple.svg" alt="Plante Patr" />
				<div class="fc-ct-ct br-sm">
					<img src="/images/patr.svg" alt="Patr Logo" />
				</div>
			</div>

			<div class="full-width full-height fc-fs-fs of-hidden pt-md">
				<nav class="full-height full-width fc-fs-fs ofy-auto mt-md">
					<ul class="full-width full-height fc-fs-fs">
						{
							links.into_iter().map(|link| view! {
								<SidebarItem link=link />
							}).collect_view()
						}
					</ul>
				</nav>
			</div>
		</aside>
	}
}
