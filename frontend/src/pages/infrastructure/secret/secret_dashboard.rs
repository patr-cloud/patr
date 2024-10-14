use crate::{pages::SecretCard, prelude::*};

/// The Secret List Item
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct SecretListItem {
	/// The Id of the secret
	pub id: String,
	/// The name of the secret
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
		<ContainerMain class="my-md">
			<ContainerHead>
				<PageTitleContainer
					page_title_items={vec![
						PageTitleItem {
							title: "Secret".to_owned(),
							link: None,
							icon_position: PageTitleIconPosition::None,
							variant: PageTitleVariant::Heading,
						},
					]}
					description_title={
						Some("Create and manage API keys, Database Passwords, and other
						sensitive information.".to_owned())
					}
					description_link={
						Some("https://docs.patr.cloud/features/secrets/".to_owned())
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

			<ContainerBody class="px-xxl py-xl gap-md">
				<TableDashboard
					column_grids={vec![11, 1]}
					headings={vec![
						view! { <p class="txt-sm txt-medium mr-auto">"Name"</p> }.into_view(),
						"".into_view(),
					]}

					render_rows={view! {
						<For each={move || data.get()} key={|state| state.clone().id} let:child>
							<SecretCard secret_item={child} />
						</For>
					}
						.into_view()}
				/>
			</ContainerBody>
		</ContainerMain>
	}
}
