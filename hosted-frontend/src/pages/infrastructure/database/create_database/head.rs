use crate::prelude::*;

#[component]
pub fn CreateDatabaseHeader() -> impl IntoView {
	view! {
		<ContainerHead>
			<div class="fr-sb-ct full-width">
				<div class="fc-fs-fs">
					<PageTitleContainer>
						<PageTitle icon_position={PageTitleIconPosition::End}>
							"Infrastructure"
						</PageTitle>
						<PageTitle
							icon_position={PageTitleIconPosition::End}
							variant={PageTitleVariant::SubHeading}
						>
							"Database"
						</PageTitle>
					</PageTitleContainer>
				</div>
			</div>
		</ContainerHead>
	}
}
