use crate::prelude::*;

#[component]
pub fn RunnerManageHead() -> impl IntoView {
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
							icon_position={PageTitleIconPosition::End}
						>
							"Runners"
						</PageTitle>
						<PageTitle
							variant={PageTitleVariant::SubHeading}
						>
							"{runner_name}"
						</PageTitle>
					</PageTitleContainer>

					<PageDescription
						description={"Create and manage CI Runners for automated builds.".to_string()}
						doc_link={Some("https://docs.patr.cloud/ci-cd/#choosing-a-runner".to_string())}
					/>
				</div>
			</div>
		</ContainerHead>
	}
}
