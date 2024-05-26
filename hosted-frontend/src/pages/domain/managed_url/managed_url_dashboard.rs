use crate::{pages::ManagedUrls, prelude::*};

#[component]
pub fn ManagedUrlDashboard() -> impl IntoView {
	view! {
		<ContainerMain>
			<ContainerHead>
				<div class="fr-sb-ct full-width">
					<div class="fc-fs-fs">
						<PageTitleContainer>
							<PageTitle icon_position={PageTitleIconPosition::End}>
								"Incfrastructure"
							</PageTitle>
							<PageTitle variant={PageTitleVariant::SubHeading}>
								"Managed URL"
							</PageTitle>
						</PageTitleContainer>

						<PageDescription
							description="Create and Manage URLs with ease using Patr."
							doc_link={Some(
								"https://docs.patr.cloud/features/managed-urls/".to_owned(),
							)}
						/>

					</div>
				</div>
			</ContainerHead>

			<ContainerBody class="px-xl">
				<div class="fc-fs-fs full-width full-height px-md py-xl gap-md">
					<TableDashboard
						column_grids={vec![4, 1, 4, 2, 1]}
						headings={vec![
							"Managed URL".into_view(),
							"Type".into_view(),
							"Target".into_view(),
							"".into_view(),
							"".into_view(),
						]}

						render_rows={view! {
							<ManagedUrls class="ul-light"/>
							<ManagedUrls class="ul-light"/>
							<ManagedUrls class="ul-light"/>
						}
							.into_view()}
					/>
				</div>
			</ContainerBody>
		</ContainerMain>
	}
}
