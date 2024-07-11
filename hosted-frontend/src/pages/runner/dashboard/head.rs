use crate::prelude::*;

#[component]
pub fn RunnerDashboardHead() -> impl IntoView {
	view! {
		<ContainerHead>
			<div class="fr-sb-ct full-width">
				<div class="fc-sb-fs">
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

				<div class="fr-ct-ct">
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
