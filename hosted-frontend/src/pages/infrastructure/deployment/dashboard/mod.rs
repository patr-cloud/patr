mod head;

use codee::string::FromToStringCodec;
use leptos_use::use_cookie;

pub use self::head::*;
use super::components::*;
use crate::prelude::*;

/// The Shell Outer for Deployment Page
#[component]
pub fn DeploymentPage() -> impl IntoView {
	view! {
		<ContainerMain class="w-full h-full my-md">
			<Outlet/>
		</ContainerMain>
	}
}

/// The Deployment Dashboard Page
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
		<DeploymentDashboardHead />

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
