use crate::prelude::*;

#[component]
pub fn ManageRepository() -> impl IntoView {
	view! {
		<ContainerHead>
			<div class="fr-fs-ct full-width">
				<PageTitleContainer>
					<PageTitle icon_position=PageTitleIconPosition::End>
						"Container Registry"
					</PageTitle>
					<PageTitle variant=PageTitleVariant::SubHeading>
						"Repo Name"
					</PageTitle>
				</PageTitleContainer>
			</div>
			<Tabs
				tab_items=vec![
					TabItem {
						name: "General".to_owned(),
						path: "".to_owned()
					},
					TabItem {
						name: "Images".to_owned(),
						path: "/images".to_owned()
					},
				]
			/>
		</ContainerHead>
		<ContainerBody class="px-xxl pt-xl pb-sm gap-md">
			<Outlet />
		</ContainerBody>
	}
}
