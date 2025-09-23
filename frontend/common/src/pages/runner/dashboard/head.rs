use crate::prelude::*;

/// The Deployment Dashboard Header
#[component]
pub fn RunnerDashboardHead() -> impl IntoView {
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
						title: "Runners".to_owned(),
						link: None,
						icon_position: PageTitleIconPosition::None,
						variant: PageTitleVariant::SubHeading,
					},
				]}
				description_title={
					Some("List of runners".to_owned())
				}
				description_link={
					Some("https://docs.patr.cloud/features/deployments/".to_owned())
				}
				action_buttons={|| view! {
					<Link
						to={"create".to_owned()}
						variant={LinkStyleVariant::Contained}
					>
						"CREATE RUNNER"
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
