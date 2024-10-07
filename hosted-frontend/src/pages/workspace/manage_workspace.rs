use crate::prelude::*;

#[component]
pub fn ManageWorkspace() -> impl IntoView {
	view! {
		<ContainerHead>
			<PageTitleContainer
				page_title_items={vec![
					PageTitleItem {
						title: "Workspace".to_owned(),
						link: None,
						icon_position: PageTitleIconPosition::None,
						variant: PageTitleVariant::Heading,
					},
				]}
				action_buttons={Some(view! {
					<Link
						class="gap-xxs"
						r#type={Variant::Link}
						style_variant={LinkStyleVariant::Contained}
						to="create"
					>
						"CREATE WORKSPACE"
						<Icon icon={IconType::Plus} size={Size::ExtraSmall} color={Color::Black} />
					</Link>
				}.into_view())}
			/>

			<Tabs tab_items={vec![
				TabItem {
					name: "Details".to_owned(),
					path: "".to_owned(),
				},
				TabItem {
					name: "Billing".to_owned(),
					path: "".to_owned(),
				},
				TabItem {
					name: "Pricing".to_owned(),
					path: "".to_owned(),
				},
				TabItem {
					name: "Transactions".to_owned(),
					path: "".to_owned(),
				},
			]} />
		</ContainerHead>

		<ContainerBody class="px-xl ofy-auto">
			<Outlet />
		</ContainerBody>
	}
}
