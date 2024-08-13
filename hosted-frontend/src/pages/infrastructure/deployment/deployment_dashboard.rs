use codee::string::FromToStringCodec;
use leptos_use::use_cookie;

use crate::{pages::DeploymentCard, prelude::*};

#[component]
pub fn Deployment() -> impl IntoView {
	view! {
		<ContainerMain class="w-full h-full mb-md">
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

	view! {
		<ContainerHead>
			<div class="flex justify-between items-center w-full">
				<div class="flex flex-col items-start justify-start">
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

				<Link
					r#type={Variant::Link}
					to={"create".to_string()}
					style_variant={LinkStyleVariant::Contained}
				>
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
						<Transition
							fallback=move || view! {<p>"loading"</p>}
						>
							{
								move || match deployment_list.get() {
									Some(Ok(data)) => {
										view! {
											<For
												each={move || data.deployments.clone()}
												key={|state| state.id}
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
