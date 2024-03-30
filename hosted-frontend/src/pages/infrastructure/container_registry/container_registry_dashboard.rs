use super::container_registry_item::ContainerRegistryCard;
use crate::prelude::*;

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct ContainerRegistryItem {
	pub id: String,
	pub name: String,
	pub size: String,
	pub date_created: String,
}

#[component]
pub fn ContainerRegistry() -> impl IntoView {
	view! {
		<ContainerMain>
			<Outlet />
		</ContainerMain>
	}
}

#[component]
pub fn ContainerRegistryDashboard() -> impl IntoView {
	let data = create_rw_signal(vec![
		ContainerRegistryItem {
			id: "12".to_owned(),
			name: "smart-tools-demo".to_owned(),
			size: "2 GB".to_owned(),
			date_created: "3 Months Ago".to_owned(),
		},
		ContainerRegistryItem {
			id: "13".to_owned(),
			name: "patr-website".to_owned(),
			size: "4.91 GB".to_owned(),
			date_created: "8 Months Ago".to_owned(),
		},
		ContainerRegistryItem {
			id: "123".to_owned(),
			name: "docker-registry".to_owned(),
			size: "26.32 MB".to_owned(),
			date_created: "1 Year Ago".to_owned(),
		},
	]);

	view! {
		<ContainerHead>
			<div class="fr-sb-ct full-width">
				<div class="fc-fs-fs">
					<PageTitleContainer>
						<PageTitle>
							"Container Registry"
						</PageTitle>
					</PageTitleContainer>

					<PageDescription
						description="Create and manage your Repositories on our private, secure
						in-built Docker Registry."
						doc_link=Some("https://docs.patr.cloud/features/container-registry/".to_owned())
					/>
				</div>

				<Link
					r#type=Variant::Button
					style_variant=LinkStyleVariant::Contained
				>
					"CREATE REPOSITORY"
					<Icon
						icon=IconType::ChevronRight
						size=Size::ExtraSmall
						class="ml-xs"
						color=Color::Black
					/>
				</Link>
			</div>
		</ContainerHead>

		<ContainerBody class="px-xxl py-xl gap-md">
			<TableDashboard
				column_grids=vec![5, 2, 4, 1]
				headings=vec![
					"Repository".into_view(),
					"Size".into_view(),
					"Date Created".into_view(),
					"".into_view()
				]
				render_rows=view! {
					<For
						each=move || data.get()
						key=|state| state.id.clone()
						let:child
					>
						<ContainerRegistryCard item=child />
					</For>
				}.into_view()
			/>
		</ContainerBody>
	}
}
