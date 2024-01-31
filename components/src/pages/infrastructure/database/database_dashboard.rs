use crate::{imports::*, pages::DatabaseCard};

#[component]
pub fn DatabaseDashboard() -> impl IntoView {
	let data = create_rw_signal(vec![0, 1, 2]);

	view! {
		<ContainerMain class="full-width full-height mb-md">
			<ContainerHead>
				<div class="fr-sb-ct full-width">
					<div class="fc-fs-fs">
							<PageTitleContainer>
								<PageTitle icon_position=PageTitleIconPosition::End>
									"Infrastructure"
								</PageTitle>
								<PageTitle variant=PageTitleVariant::SubHeading>
									"Database"
								</PageTitle>
							</PageTitleContainer>

							<PageDescription
								description="Create and manage Databases using Patr."
								doc_link=Some("https://docs.patr.cloud/features/databases/".to_owned())
							/>
					</div>

					<Link r#type=Variant::Button style_variant=LinkStyleVariant::Contained>
						"ADD DATABASE"
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
				<div></div>
				<DashboardContainer
					gap=Size::Large
					render_items=view! {
						<For
							each=move || data.get()
							key=|state| state.clone()
							let:child
						>
							<DatabaseCard />
						</For>
					}
				/>
			</ContainerBody>
		</ContainerMain>
	}
}
