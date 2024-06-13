use crate::prelude::*;

#[component]
pub fn ManageWorkspace() -> impl IntoView {
	view! {
		<ContainerHead>
			<div class="fr-sb-ct full-width">
				<PageTitleContainer>
					<PageTitle>"Workspace"</PageTitle>
				</PageTitleContainer>

				<Link
					class="gap-xxs"
					r#type=Variant::Link
					style_variant=LinkStyleVariant::Contained
					to="create"
				>
					"CREATE WORKSPACE"
					<Icon icon=IconType::Plus size=Size::ExtraSmall color=Color::Black />
				</Link>
			</div>

			<Tabs tab_items={vec![
				TabItem {
					name: "Details".to_owned(),
					path: "".to_owned()
				},
				TabItem {
					name: "Billing".to_owned(),
					path: "".to_owned()
				},
				TabItem {
					name: "Pricing".to_owned(),
					path: "".to_owned()
				},
				TabItem {
					name: "Transactions".to_owned(),
					path: "".to_owned()
				}
			]}/>
		</ContainerHead>

		<ContainerBody class="px-xl ofy-auto">
			<Outlet />
		</ContainerBody>
	}
}
