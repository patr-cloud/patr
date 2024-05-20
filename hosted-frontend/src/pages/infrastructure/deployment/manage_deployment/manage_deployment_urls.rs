use crate::{pages::ManagedUrlCard, prelude::*};

#[component]
pub fn ManageDeploymentUrls() -> impl IntoView {
	view! {
		<div class="pt-xl px-xl fr-fe-ct full-width">
			<Link
				// r#type=Variant::Button
				// style_variant=LinkStyleVariant::Contained
			>
				"CREATE MANAGED URL"
				<Icon icon=IconType::Plus size=Size::ExtraSmall color=Color::Secondary />
			</Link>
		</div>
		<TableDashboard
			class="px-xl"
			column_grids=[4, 1, 4, 2, 1]
			headings=vec![
				"Managed URL".into_view(),
				"Type".into_view(),
				"Target".into_view(),
				"".into_view(),
				"".into_view(),
			]
			render_rows=view! {
				<ManagedUrlCard />
			}.into_view()
		/>
	}
}
