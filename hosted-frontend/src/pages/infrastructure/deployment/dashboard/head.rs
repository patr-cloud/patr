use crate::prelude::*;

#[component]
pub fn DeploymentDashboardHead() -> impl IntoView {
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
						title: "Deployment".to_owned(),
						link: None,
						icon_position: PageTitleIconPosition::None,
						variant: PageTitleVariant::SubHeading,
					},
				]}
				description_title={
					Some("Create and Manage Deployments with ease using Patr".to_owned())
				}
				description_link={
					Some("https://docs.patr.cloud/features/deployments/".to_owned())
				}
				action_buttons={Some(view! {
					<Link
						r#type={Variant::Link}
						to={"create".to_owned()}
						style_variant={LinkStyleVariant::Contained}
					>
						"CREATE DEPLOYMENT"
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
