use crate::{pages::SecretCard, prelude::*};

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct SecretListItem {
	pub id: String,
	pub name: String,
}

#[component]
pub fn SecretsDashboard() -> impl IntoView {
	let data = create_rw_signal(vec![
		SecretListItem {
			id: "1244".to_owned(),
			name: "Email".to_owned(),
		},
		SecretListItem {
			id: "13".to_owned(),
			name: "Password".to_owned(),
		},
		SecretListItem {
			id: "123".to_owned(),
			name: "Twilio Auth Key".to_owned(),
		},
	]);
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
					column_grids=vec![11, 1]
					headings=vec![
						view! {
							<p class="txt-sm txt-medium mr-auto">"Name"</p>
						}.into_view(),
						"".into_view()
					]
					render_rows=view! {
						<For
							each=move || data.get()
							key=|state| state.id
							let:child
						>
							<SecretCard secret_item=child />
						</For>
					}.into_view()
				/>
			</ContainerBody>
		</ContainerMain>
	}
}
