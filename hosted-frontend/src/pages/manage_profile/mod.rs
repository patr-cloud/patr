use crate::prelude::*;

#[component]
pub fn ManageProfile() -> impl IntoView {
	view! {
		<ContainerMain class="full-width full-height mb-md">
			<ContainerHead>
				<PageTitleContainer>
					<PageTitle>"Manage Profile"</PageTitle>

				</PageTitleContainer>

				<Tabs
					tab_items=vec![
						TabItem {
							name: "Settings".to_owned(),
							path: "/settings".to_owned()
						},
						TabItem {
							name: "API Tokens".to_owned(),
							path: "/tokens".to_owned()
						},
					]
				/>
			</ContainerHead>

			<ContainerBody class="gap-md">
				<ApiTokensTab />
			</ContainerBody>
		</ContainerMain>
	}
}

mod tabs;

pub use self::tabs::*;
