use leptos_use::{use_cookie, utils::FromToStringCodec};

use crate::{pages::DeploymentCard, prelude::*};

/// Deployment Model
/// TO BE REPLACED LATER WITH MODEL A PROPER MODEL TYPE
/// ACCORDING TO THE REQUEST RESPONSE TYPE
#[derive(PartialEq, Eq, Clone)]
pub struct DeploymentType {
	/// The Id of the deployment
	pub id: String,
	/// The Name of the deployment
	pub name: String,
	/// The Image Tag of the deployment
	pub image_tag: String,
	/// The Status of the deployment
	pub status: Status,
	/// The Region of the deployment
	pub region: String,
	/// The Machine Type of the deployment
	pub machine_type: String,
}

#[component]
pub fn Deployment() -> impl IntoView {
	view! {
		<ContainerMain class="full-width full-height mb-md">
			<Outlet/>
		</ContainerMain>
	}
}

#[component]
pub fn DeploymentDashboard() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let deployment_list = create_resource(
		move || (access_token.get(), current_workspace_id.get()),
		move |(access_token, workspace_id)| async move {
			list_deployments(workspace_id, access_token).await
		},
	);

	create_effect(move |_| logging::log!("{:#?}", deployment_list.get()));

	view! {
		<ContainerHead>
			<div class="fr-sb-ct full-width">
				<div class="fc-fs-fs">
					<PageTitleContainer>
						<PageTitle icon_position={PageTitleIconPosition::End}>
							"Infrastructure"
						</PageTitle>
						<PageTitle variant={PageTitleVariant::SubHeading}>"Deployment"</PageTitle>
					</PageTitleContainer>

					<PageDescription
						description="Create and Manage Deployments with ease using Patr."
						doc_link={Some("https://docs.patr.cloud/features/deployments/".to_owned())}
					/>
				</div>

				<Link r#type={Variant::Button} style_variant={LinkStyleVariant::Contained}>
					"CREATE DEPLOYMENT"
					<Icon
						icon={IconType::Plus}
						size={Size::ExtraSmall}
						class="ml-xs"
						color={Color::Black}
					/>
				</Link>
			</div>
		</ContainerHead>

		<ContainerBody>
			<DashboardContainer
				gap={Size::Large}
				render_items={
					view! {
						<Transition>
							{
								move || match deployment_list.get() {
									Some(Ok(data)) => {
										view! {
											<For
												each={move || data.deployments.clone()}
												key={|state| state.id.clone()}
												let:child
											>
												<DeploymentCard deployment={child}/>
											</For>
										}
									},
									_ => view! {}.into_view()
								}
							}
						</Transition>
					}
				}
			/>

		</ContainerBody>
	}
}
