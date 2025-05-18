use crate::prelude::*;

/// The Deployment Dashboard Header
#[component]
pub fn DeploymentDashboardHead() -> Element {
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
				action_buttons={|| view! {
					<Link
						to={"create".to_owned()}
						variant={LinkStyleVariant::Contained}
					>
						"CREATE DEPLOYMENT"
						<Icon
							icon={IconType::Plus}
							size={Size::ExtraSmall}
							class="ml-xs"
							color={Color::Black}
						/>
					</Link>
				}}
			/>
		</ContainerHead>
	}
}
