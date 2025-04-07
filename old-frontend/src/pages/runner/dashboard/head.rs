use crate::prelude::*;

/// The Runner Dashboard Heading
#[component]
pub fn RunnerDashboardHead() -> impl IntoView {
	view! {
		<ContainerHead>
			<PageTitleContainer
				page_title_items={vec![
					PageTitleItem {
						title: "CI/CD".to_owned(),
						link: None,
						icon_position: PageTitleIconPosition::End,
						variant: PageTitleVariant::Heading,
					},
					PageTitleItem {
						title: "Runners".to_owned(),
						link: Some("/runners".to_owned()),
						icon_position: PageTitleIconPosition::None,
						variant: PageTitleVariant::SubHeading,
					},
				]}
				description_title={
					Some("Create and manage CI Runners for automated builds.".to_owned())
				}
				description_link={
					Some("https://docs.patr.cloud/ci-cd/#choosing-a-runner".to_owned())
				}
				action_buttons={Some(view! {
					<Link
						r#type={Variant::Link}
						to={"create".to_owned()}
						style_variant={LinkStyleVariant::Contained}
					>
						"CREATE RUNNER"
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
