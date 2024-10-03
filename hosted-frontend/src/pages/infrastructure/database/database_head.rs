use crate::prelude::*;

#[component]
pub fn DatabaseHead() -> impl IntoView {
	view! {
		<ContainerHead>
			<PageTitleContainer
				page_title_items={vec![
					PageTitleItem {
						title: "Infrastructure".to_owned(),
						link: None,
						icon_position: PageTitleIconPosition::End,
						variant: PageTitleVariant::Heading,
					},
					PageTitleItem {
						title: "Database".to_owned(),
						link: None,
						icon_position: PageTitleIconPosition::None,
						variant: PageTitleVariant::SubHeading,
					},
				]}
				description_title={
					Some("Create and manage Databases using Patr".to_owned())
				}
				description_link={
					Some("https://docs.patr.cloud/features/databases/".to_owned())
				}
				action_buttons={Some(view! {
					<Link
						r#type={Variant::Link}
						to="create"
						style_variant={LinkStyleVariant::Contained}
					>
						"CREATE DATABASE"
						<Icon
							icon={IconType::Plus}
							size={Size::ExtraSmall}
							class="ml-xs"
							color={Color::Black}
						/>
					</Link>
				}.into_view())}
			/>
		</ContainerHead>
	}
}
