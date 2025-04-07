use crate::prelude::*;

#[component]
pub fn RunnerCreateHead() -> impl IntoView {
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
						title: "Runner".to_owned(),
						link: None,
						icon_position: PageTitleIconPosition::End,
						variant: PageTitleVariant::SubHeading,
					},
					PageTitleItem {
						title: "New".to_owned(),
						link: None,
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
			/>
		</ContainerHead>
	}
}
