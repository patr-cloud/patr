use crate::{pages::DomainCard, prelude::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub enum DomainNameServerType {
	External,
	#[default]
	Internal,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct DomainItemType {
	pub id: String,
	pub name: String,
	pub name_server: DomainNameServerType,
	pub verified: bool,
}

#[component]
pub fn DomainsDashboard() -> impl IntoView {
	let data = create_rw_signal(vec![
		DomainItemType {
			id: "124".to_owned(),
			name: "onpatr.cloud".to_owned(),
			name_server: DomainNameServerType::Internal,
			verified: true,
		},
		DomainItemType {
			id: "1324".to_owned(),
			name: "external.site".to_owned(),
			name_server: DomainNameServerType::External,
			verified: false,
		},
	]);
	view! {
		<ContainerMain>
			<ContainerHead>
				<div class="fr-sb-ct full-width">
					<div class="fc-fs-fs">
						<PageTitleContainer>
							<PageTitle>
								"Domain"
							</PageTitle>
						</PageTitleContainer>

						<PageDescription
							description="Connect and Manage Domains through Patr."
							doc_link=Some("https://docs.patr.cloud/features/domains/".to_owned())
						/>
					</div>

					<Link
						r#type=Variant::Button
						style_variant=LinkStyleVariant::Contained
					>
						"ADD DOMAIN"
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
					column_grids=vec![4, 3, 3]
					headings=vec![
						"Domain".into_view(),
						"Name Server".into_view(),
						"Verification Status".into_view()
					]
					render_rows=view! {
						<For
							each=move || data.get()
							key=|state| state.id.clone()
							let:child
						>
							<DomainCard domain_item=child />
						</For>
					}.into_view()
				/>
			</ContainerBody>
		</ContainerMain>
	}
}
