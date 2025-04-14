use crate::{pages::StaticSiteCard, prelude::*};

/// The Model for the static site item
/// TO BE REPLACED WITH A PROPER MODEL TYPE
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct StaticSiteItemType {
	/// The Id of the site
	pub id: String,
	/// The Name of the deployed site
	pub name: String,
	/// The status of the site
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
		<ContainerMain class="my-md">
			<ContainerHead>
				<PageTitleContainer
					page_title_items={vec![
						PageTitleItem {
							title: "Infrastructure".to_owned(),
							link: None,
							icon_position: PageTitleIconPosition::End,
							variant: PageTitleVariant::Heading,
						},
						PageTitleItem {
							title: "Static Site".to_owned(),
							link: None,
							icon_position: PageTitleIconPosition::None,
							variant: PageTitleVariant::SubHeading,
						},
					]}
					description_title={
						Some("Deploy And Manage Static Sites using Patr".to_owned())
					}
					description_link={
						Some("https://docs.patr.cloud/features/static-sites/".to_owned())
					}
					action_buttons={Some(view! {
						<Link
							r#type={Variant::Button}
							style_variant={LinkStyleVariant::Contained}
						>
							"CREATE SECRET"
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

			<ContainerBody>
				<DashboardContainer
					gap={Size::Large}
					render_items={view! {
						<For each={move || data.get()} key={|state| state.id.clone()} let:child>
							<StaticSiteCard static_site={child} />
						</For>
					}
						.into_view()}
				/>
			</ContainerBody>

		</ContainerMain>
	}
}
