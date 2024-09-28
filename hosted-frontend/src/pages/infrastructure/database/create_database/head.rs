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
							to="/database"
						>
							"Database"
						</PageTitle>
						<PageTitle variant={PageTitleVariant::SubHeading}>"Create"</PageTitle>
					</PageTitleContainer>

					<PageDescription
						description={"Create a new Database here.".to_string()}
						doc_link={Some("https://docs.patr.cloud/features/databases/".to_string())}
					/>
				</div>
			</div>
		</ContainerHead>
	}
}
