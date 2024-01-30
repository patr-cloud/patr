use crate::imports::*;

#[component]
pub fn SecretsDashboard() -> impl IntoView {
	view! {
		<ContainerMain>
			<ContainerHead>
				<div class="fr-sb-ct full-width">
					<div class="fc-fs-fs">
						<PageTitleContainer>
							<PageTitle>
								"Secret"
							</PageTitle>
						</PageTitleContainer>

						<PageDescription
							description="Create and manage API keys, Database Passwords, and other
								sensitive information."
							doc_link=Some("https://docs.patr.cloud/features/secrets/".to_owned())
						/>
					</div>

					<Link
						r#type=Variant::Button
						style_variant=LinkStyleVariant::Contained
					>
						"CREATE SECRET"
						<Icon
							icon=IconType::Plus
							size=Size::ExtraSmall
							class="ml-xs"
							color=Color::Black
						/>
					</Link>
				</div>
			</ContainerHead>

			<ContainerBody class="px-xxl py-xl gap-md">
				<TableDashboard
					headings=vec![
						view! {
							<p class="txt-sm txt-medium mr-auto">"Name"</p>
						}.into_view(),
						"".into_view()
					]
				/>
			</ContainerBody>
		</ContainerMain>
	}
}
