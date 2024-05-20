use crate::prelude::*;

#[component]
pub fn EditRepository() -> impl IntoView {
	view! {
		<form class="full-width px-md mb-lg full-height fc-fs-fs fit-wide-screen mx-auto">
			<div class="flex full-width my-xs">
				<label
					html_for="repo-name"
					class="txt-white txt-sm flex-col-2 fr-fs-fs mt-sm"
				>
					"Repository Name"
				</label>
				<div class="flex-col-10 fc-fs-fs gap-xs">
					<Input
						r#type=InputType::Text
						placeholder="Enter Name"
						class="full-width"
						value="registry.patr.cloud/def74e7222034b5ca0ee2cb4cee585cd/{container_registry_name}"
						disabled=true
					/>
				</div>
			</div>

			<div class="flex full-width my-xs">
				<label
					html_for="repo-name"
					class="txt-white txt-sm flex-col-2 fr-fs-fs mt-sm"
				>
					"Size"
				</label>
				<div class="flex-col-10 fc-fs-fs gap-xs">
					<Input
						r#type=InputType::Text
						placeholder="Size"
						class="full-width"
						value="2.47 GB"
						disabled=true
					/>
				</div>
			</div>

			<div class="flex full-width my-xs">
				<label
					html_for="repo-name"
					class="txt-white txt-sm flex-col-2 fr-fs-fs mt-sm"
				>
					"Last Updated"
				</label>
				<div class="flex-col-10 fc-fs-fs gap-xs">
					<Input
						r#type=InputType::Text
						placeholder="Size"
						class="full-width"
						value="5 Months Ago"
						disabled=true
					/>
				</div>
			</div>

			<div class="fr-fs-ct mt-auto ml-auto">
				<Link
					class="mr-xs btn"
				>
					"BACK"
				</Link>
				<Link
					style_variant=LinkStyleVariant::Contained
					should_submit=true
				>
					"CREATE"
				</Link>
			</div>
		</form>
	}
}

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
						"Repository Name"
					</PageTitle>
				</PageTitleContainer>
			</div>
			<Tabs
				tab_items=vec![
					TabItem {
						name: "General".to_owned(),
						path: "".to_owned()
					},
				]
			/>
		</ContainerHead>
		<ContainerBody class="px-xxl pt-xl pb-sm gap-md">
			<Outlet />
		</ContainerBody>
	}
}
