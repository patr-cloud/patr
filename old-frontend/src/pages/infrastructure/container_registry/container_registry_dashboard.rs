use super::container_registry_item::ContainerRegistryCard;
use crate::prelude::*;

/// A Container Registry Item
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct ContainerRegistryItem {
	/// The Id of the Container Registry Item
	pub id: String,
	/// The Name of the Container Registry Item
	pub name: String,
	/// The Size of the Container Registry Item
	pub size: String,
	/// The Date of creation of the Container Registry Item
	pub date_created: String,
}

#[component]
pub fn ContainerRegistry() -> impl IntoView {
	view! {
		<ContainerMain class="my-md">
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
			<PageTitleContainer
				page_title_items={vec![
					PageTitleItem {
						title: "Container Registry".to_owned(),
						link: None,
						icon_position: PageTitleIconPosition::None,
						variant: PageTitleVariant::Heading,
					},
				]}
				description_title={
					Some("Create and manage your Repositories on our private, secure
					in-built Docker Registry.".to_owned())
				}
				description_link={
					Some("https://docs.patr.cloud/features/container-registry/".to_owned())
				}
				action_buttons={Some(view! {
					<Link
						r#type={Variant::Button}
						style_variant={LinkStyleVariant::Contained}
					>
						"CREATE REPOSITORY"
						<Icon
							icon={IconType::Plus}
							size={Size::ExtraSmall}
							class="ml-xs"
							color={Color::Black}
						/>
					</Link>
				}.into_view())}
			/>
		</ContainerHead>

		<ContainerBody class="px-xxl py-xl gap-md">
			<TableDashboard
				column_grids={vec![5, 2, 4, 1]}
				headings={vec![
					"Repository".into_view(),
					"Size".into_view(),
					"Date Created".into_view(),
					"".into_view(),
				]}

				render_rows={view! {
					<For each={move || data.get()} key={|state| state.id.clone()} let:child>
						<ContainerRegistryCard item={child} />
					</For>
				}
					.into_view()}
			/>
		</ContainerBody>
	}
}
