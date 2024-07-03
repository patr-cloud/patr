use crate::prelude::*;

#[component]
pub fn CreateDeploymentHead() -> impl IntoView {
	view! {
		<ContainerHead>
			<div class="fr-sb-ct full-width">
				<div class="fc-fs-fs">
					<PageTitleContainer>
						<PageTitle icon_position={PageTitleIconPosition::End}>
							"Infrastructure"
						</PageTitle>
						<PageTitle
							to="deployment"
							icon_position={PageTitleIconPosition::End}
							variant={PageTitleVariant::SubHeading}
						>
							"Deployment"
						</PageTitle>
						<PageTitle variant={PageTitleVariant::Text}>"Create Deployment"</PageTitle>
					</PageTitleContainer>

					<PageDescription
						description={"Create a new Deployment here.".to_string()}
						doc_link={Some("https://docs.patr.cloud/features/deployments/#how-to-create-a-deployment".to_string())}
					/>
				</div>
			</div>
		</ContainerHead>
	}
}
