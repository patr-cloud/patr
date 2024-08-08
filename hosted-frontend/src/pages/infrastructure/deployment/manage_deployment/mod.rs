mod details;
mod head;
mod image_history;
mod image_history_card;
mod image_tag;
mod logs;
mod monitor;
mod scaling;
mod urls;

use models::api::workspace::deployment::*;
use codee::string::FromToStringCodec;

pub use self::{
	details::*,
	head::*,
	image_history::*,
	image_history_card::*,
	image_tag::*,
	logs::*,
	monitor::*,
	scaling::*,
	urls::*,
};
use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct DeploymentInfoContext(RwSignal<Option<GetDeploymentInfoResponse>>);

#[derive(Params, PartialEq)]
pub struct DeploymentParams {
	deployment_id: Option<String>,
}

#[component]
pub fn ManageDeployments() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let params = use_params::<DeploymentParams>();
	let deployment_id = Signal::derive(move || {
		params.with(|params| {
			params
				.as_ref()
				.map(|param| param.deployment_id.clone())
				.unwrap_or_default()
		})
	});

	let deployment_info = create_resource(
		move || {
			(
				access_token.get(),
				deployment_id.get(),
				current_workspace_id.get(),
			)
		},
		move |(access_token, deployment_id, workspace_id)| async move {
			get_deployment(access_token, deployment_id, workspace_id).await
		},
	);

	let deployment_info_signal = create_rw_signal::<Option<GetDeploymentInfoResponse>>(None);
	provide_context(DeploymentInfoContext(deployment_info_signal));

	view! {
		<Transition>
			{
				move || match deployment_info.get() {
					Some(info) => {
						match info {
							Ok(data) => {
								let deployment = data.clone();
								logging::log!("{:#?}", deployment);
								deployment_info_signal.set(Some(deployment.clone()));
								view! {
									<ManageDeploymentHeader />
									<ContainerBody class="gap-md">
										<Outlet/>
									</ContainerBody>
								}.into_view()
							},
							Err(_)  => view! {
								<div>"Error Fetching Resource"</div>
							}.into_view(),
						}
					},
					None => view! {<div>"Loading"</div>}.into_view()
				}
			}
		</Transition>

	}
}
