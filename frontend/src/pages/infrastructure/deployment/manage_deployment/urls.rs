use crate::prelude::*;

#[component]
pub fn ManageDeploymentUrls() -> impl IntoView {
	view! {
		<div class="pt-xl px-xl flex justify-end items-center w-full">
			<Link>
				// r#type=Variant::Button
				// style_variant=LinkStyleVariant::Contained
				"CREATE MANAGED URL"
				<Icon icon={IconType::Plus} size={Size::ExtraSmall} color={Color::Secondary} />
			</Link>
		</div>
		<TableDashboard
			class="px-xl"
			column_grids={[4, 1, 4, 2, 1]}
			headings={vec![
				"Managed URL".into_any(),
				"Type".into_any(),
				"Target".into_any(),
				"".into_any(),
				"".into_any(),
			]}

			render_rows={view! { <div></div> }.into_any()}
		/>
	}
}
