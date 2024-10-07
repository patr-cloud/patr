use crate::{pages::DomainCard, prelude::*};

/// The Type of the Domain Name Server, Whether we're using an internal server
/// or an external one
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub enum DomainNameServerType {
	/// External Domain Name Server
	External,
	/// Internal Domain Name Server
	#[default]
	Internal,
}

/// Domain Model
/// TO BE REPLACED LATER WITH MODEL A PROPER MODEL TYPE
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct DomainItemType {
	/// The Id of the domain
	pub id: String,
	/// The name of the domain
	pub name: String,
	/// The Name server type
	pub name_server: DomainNameServerType,
	/// Whether the domain is verified
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
		<ContainerMain class="my-md">
			<ContainerHead>
				<PageTitleContainer
					page_title_items={vec![
						PageTitleItem {
							title: "Domain".to_owned(),
							link: None,
							icon_position: PageTitleIconPosition::None,
							variant: PageTitleVariant::Heading,
						}
					]}
					description_title={
						Some("Connect and Manage Domains through Patr.".to_owned())
					}
					description_link={
						Some("https://docs.patr.cloud/features/domains/".to_owned())
					}
					action_buttons={Some(view! {
						<Link r#type={Variant::Button} style_variant={LinkStyleVariant::Contained}>
							"ADD DOMAIN"
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
					column_grids={vec![4, 3, 3]}
					headings={vec![
						"Domain".into_view(),
						"Name Server".into_view(),
						"Verification Status".into_view(),
					]}

					render_rows={view! {
						<For each={move || data.get()} key={|state| state.id.clone()} let:child>
							<DomainCard domain_item={child} />
						</For>
					}
						.into_view()}
				/>
			</ContainerBody>
		</ContainerMain>
	}
}
