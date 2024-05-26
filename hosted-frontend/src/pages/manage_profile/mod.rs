use crate::prelude::*;

#[component(transparent)]
pub fn ProfileRoutes() -> impl IntoView {
	view! {
		<Route path={LoggedInRoute::Profile} view={ManageProfile}>
			<Route path={LoggedInRoute::ApiTokens} view={ApiTokensTab}/>
			<Route path={AppRoutes::Empty} view={ProfileSettings}/>
		</Route>
	}
}

#[component]
pub fn ManageProfile() -> impl IntoView {
	view! {
		<ContainerMain class="full-width full-height mb-md">
			<ContainerHead>
				<PageTitleContainer>
					<PageTitle>"Manage Profile"</PageTitle>
				</PageTitleContainer>

				<Tabs tab_items={vec![
					TabItem {
						name: "Settings".to_owned(),
						path: "".to_owned(),
					},
					TabItem {
						name: "API Tokens".to_owned(),
						path: "api-tokens".to_owned(),
					},
				]}/>

			</ContainerHead>

			<ContainerBody class="gap-md">
				<Outlet/>
			</ContainerBody>
		</ContainerMain>
	}
}

mod tabs;

pub use self::tabs::*;
