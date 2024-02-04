use crate::{prelude::*, pages::StaticSiteCard};

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct StaticSiteItemType {
	pub id: String,
	pub name: String,
	pub status: Status,
}

#[component]
pub fn StaticSiteDashboard() -> impl IntoView {
	let data = create_rw_signal(vec![
		StaticSiteItemType {
			id: "1213123qa".to_owned(),
			name: "Site One".to_owned(),
			status: Status::Created,
		},
		StaticSiteItemType {
			id: "23423423".to_owned(),
			name: "Site Two".to_owned(),
			status: Status::Created,
		},
		StaticSiteItemType {
			id: "67453454".to_owned(),
			name: "Site Three".to_owned(),
			status: Status::Created,
		},
	]);
	view! {
		<ContainerMain>
			<ContainerHead>
				<div class="fr-sb-ct full-width">
					<div class="fc-fs-fs">
						<PageTitleContainer>
							<PageTitle icon_position=PageTitleIconPosition::End>
								"Infrastructure"
							</PageTitle>
							<PageTitle variant=PageTitleVariant::SubHeading>
								"Static Site"
							</PageTitle>
						</PageTitleContainer>

						<PageDescription
							description="Deploy And Manage Static Sites using Patr"
							doc_link=Some("https://docs.patr.cloud/features/static-sites/".to_owned())
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

			<ContainerBody>
				<DashboardContainer
					gap=Size::Large
					render_items=view! {
						<For
							each=move || data.get()
							key=|state| state.id.clone()
							let:child
						>
							<StaticSiteCard static_site=child />
						</For>
					}.into_view()
				/>
			</ContainerBody>

		</ContainerMain>
	}
}
