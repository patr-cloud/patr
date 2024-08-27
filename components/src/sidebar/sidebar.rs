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
) -> impl IntoView {
	let links: Vec<LinkItem> = vec![
		LinkItem {
			title: "Home".to_owned(),
			path: "/".to_owned(),
			icon_src: "/images/sidebar/home.svg".to_owned(),
			subtitle: None,
			items: None,
		},
		LinkItem {
			title: "Runners".to_owned(),
			path: "/runners".to_owned(),
			icon_src: "/images/sidebar/runner.svg".to_owned(),
			subtitle: None,
			items: None,
		},
		LinkItem {
			title: "Infrastructure".to_owned(),
			path: "".to_owned(),
			icon_src: "/images/sidebar/infrastructure.svg".to_owned(),
			subtitle: None,
			items: Some(vec![
				LinkItem {
					title: "Deployments".to_owned(),
					path: "/deployment".to_owned(),
					subtitle: None,
					icon_src: "/images/sidebar/deployment.svg".to_owned(),
					items: None,
				},
				LinkItem {
					title: "Databases".to_owned(),
					path: "/database".to_owned(),
					subtitle: None,
					icon_src: "/images/sidebar/database.svg".to_owned(),
					items: None,
				},
			]),
		},
		LinkItem {
			title: "Domain Configuration".to_owned(),
			path: "".to_owned(),
			icon_src: "/images/sidebar/domains.svg".to_owned(),
			subtitle: None,
			items: Some(vec![
				LinkItem {
					title: "Domains".to_owned(),
					path: "/domain".to_owned(),
					subtitle: None,
					icon_src: "/images/sidebar/domains.svg".to_owned(),
					items: None,
				},
				LinkItem {
					title: "Managed URLs".to_owned(),
					icon_src: "/images/sidebar/managed-url.svg".to_owned(),
					subtitle: None,
					path: "/managed-url".to_owned(),
					items: None,
				},
			]),
		},
		LinkItem {
			title: "User".to_owned(),
			path: "/user".to_owned(),
			icon_src: "/images/sidebar/workspace.svg".to_owned(),
			subtitle: None,
			items: Some(vec![
				LinkItem {
					title: "Workspace".to_owned(),
					icon_src: "/images/sidebar/workspace.svg".to_owned(),
					subtitle: None,
					path: "/workspace".to_owned(),
					items: None,
				},
				LinkItem {
					title: "API Tokens".to_owned(),
					icon_src: "/images/sidebar/secrets.svg".to_owned(),
					subtitle: None,
					path: "/user/api-tokens".to_owned(),
					items: None,
				},
			]),
		},
	];

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
						{links
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
