use crate::prelude::*;

mod tabs;

pub use self::tabs::*;

/// All the Routes for Profile Pages,
/// Contains create and edit api tokens, and
/// Managed Profile
#[component(transparent)]
pub fn ProfileRoutes() -> impl IntoView {
	let app_type = expect_context::<AppType>();

	view! {
		<Route path={LoggedInRoute::UserProfile} view={ManageProfile}>
			{app_type
				.is_managed()
				.then(|| {
					view! {
						<Route path={LoggedInRoute::ApiTokens} view={ApiTokensTab}>
							<Route path="create" view={CreateApiToken} />
							<Route path=":token_id" view={EditApiToken} />
							<Route path={AppRoutes::Empty} view={ListApiTokens} />
						</Route>
					}
				})}
			<Route path={AppRoutes::Empty} view={ProfileSettings} />
		</Route>
	}
}

/// All the Outer Shell for Profile Pages, Contains the header and the tabs
#[component]
pub fn ManageProfile() -> impl IntoView {
	view! {
		<ContainerMain class="w-full h-full mb-md">
			<ContainerHead>
				<PageTitleContainer
					page_title_items={vec![
						PageTitleItem {
							title: "Manage Profile".to_owned(),
							link: None,
							icon_position: PageTitleIconPosition::None,
							variant: PageTitleVariant::Heading,
						},
					]}
				/>
				<Tabs tab_items={vec![
					TabItem {
						name: "Settings".to_owned(),
						path: "".to_owned(),
					},
					TabItem {
						name: "API Tokens".to_owned(),
						path: "api-tokens".to_owned(),
					},
				]} />

			</ContainerHead>

			<ContainerBody class="gap-md">
				<Outlet />
			</ContainerBody>
		</ContainerMain>
	}
}
