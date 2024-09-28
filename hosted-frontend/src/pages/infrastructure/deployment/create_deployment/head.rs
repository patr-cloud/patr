use crate::prelude::*;

#[component]
pub fn CreateDeploymentHead() -> impl IntoView {
	view! {
		<ContainerHead>
			<div class="w-full flex justify-between items-center">
				<div class="flex flex-col items-start justify-start">
					<PageTitleContainer>
						<PageTitle icon_position={PageTitleIconPosition::End}>
							"Infrastructure"
						</PageTitle>
						<PageTitle
							to="/deployment"
							icon_position={PageTitleIconPosition::End}
							variant={PageTitleVariant::SubHeading}
						>
							"Deployment"
						</PageTitle>
						<PageTitle variant={PageTitleVariant::Text}>"Create Deployment"</PageTitle>
					</PageTitleContainer>

					<PageDescription
						description={"Create a new Deployment here.".to_string()}
						doc_link={Some(
							"https://docs.patr.cloud/features/deployments/#how-to-create-a-deployment"
								.to_string(),
						)}
					/>
				</div>
			</div>
		</ContainerHead>
	}
}
