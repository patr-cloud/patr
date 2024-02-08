use crate::{pages::DatabaseCard, prelude::*};

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct DatabaseItem {
	pub id: i32,
	pub name: String,
	pub region: String,
	pub engine: String,
	pub version: String,
	pub plan: String,
}

#[component]
pub fn DatabaseDashboard() -> impl IntoView {
	let data = create_rw_signal(vec![
		DatabaseItem {
			id: 1234,
			name: "Mongo Database Instance".to_owned(),
			region: "aws-prod".to_owned(),
			engine: "MONGO".to_owned(),
			version: "4".to_owned(),
			plan: "1vCPU 2 GB RAM".to_owned(),
		},
		DatabaseItem {
			id: 2345,
			name: "Azure Database Instance".to_owned(),
			region: "azure-prod".to_owned(),
			engine: "PSQL".to_owned(),
			version: "2".to_owned(),
			plan: "1vCPU 2 GB RAM".to_owned(),
		},
		DatabaseItem {
			id: 3567,
			name: "Mongo Database Instance".to_owned(),
			region: "aws-prod".to_owned(),
			engine: "MariaDB".to_owned(),
			version: "1".to_owned(),
			plan: "2vCPU 4 GB RAM".to_owned(),
		},
	]);

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
				<DashboardContainer
					gap=Size::Large
					render_items=view! {
						<For
							each=move || data.get()
							key=|state| state.id
							let:child
						>
							<DatabaseCard deployment=child />
						</For>
					}
				/>
			</ContainerBody>
		</ContainerMain>
	}
}