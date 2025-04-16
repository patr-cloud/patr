use super::AppType;
use crate::components::sidebar::LinkItem;

/// Get the Sidebar Items depending on the App Type
pub fn get_sidebar_items(app_type: AppType) -> Vec<LinkItem> {
	match app_type {
		AppType::SelfHosted => vec![
			LinkItem {
				title: "Home".to_owned(),
				path: "/".to_owned(),
				icon_src: "/images/sidebar/home.svg".to_owned(),
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
				items: Some(vec![LinkItem {
					title: "API Tokens".to_owned(),
					icon_src: "/images/sidebar/secrets.svg".to_owned(),
					subtitle: None,
					path: "/user/api-tokens".to_owned(),
					items: None,
				}]),
			},
		],
		AppType::Managed => vec![
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
		],
	}
}
