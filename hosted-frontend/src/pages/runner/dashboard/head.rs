use crate::prelude::*;

/// The Runner Dashboard Heading
#[component]
pub fn RunnerDashboardHead() -> impl IntoView {
	view! {
		<ContainerHead>
			<div class="w-full flex justify-between items-center">
				<div class="flex flex-col justify-between items-start">
					<PageTitleContainer>
						<PageTitle icon_position={PageTitleIconPosition::End}>
							"CI/CD"
						</PageTitle>
						<PageTitle
							to="/runners"
							variant={PageTitleVariant::SubHeading}
						>
							"Runners"
						</PageTitle>
					</PageTitleContainer>

					<PageDescription
						description={"Create and manage CI Runners for automated builds.".to_string()}
						doc_link={Some("https://docs.patr.cloud/ci-cd/#choosing-a-runner".to_string())}
					/>
				</div>

				<div class="flex items-center justify-center">
					<Link
						r#type={Variant::Link}
						to={"create".to_string()}
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
				</div>
			</div>
		</ContainerHead>
	}
}
