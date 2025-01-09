mod details;
mod head;
mod image_history;
mod image_history_card;
mod image_tag;
mod logs;
mod monitor;
mod scaling;
mod urls;

use convert_case::*;
use models::api::workspace::deployment::*;

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
use crate::{
	pages::infrastructure::deployment::utils::DeploymentInfoContext,
	prelude::*,
	queries::get_deployment_query,
};

/// The Route Params for the manage deployments page
#[derive(Params, PartialEq)]
pub struct DeploymentParams {
	deployment_id: Option<String>,
}

#[component]
pub fn ManageDeploymentsContent(
	/// The deployment id
	#[prop(into)]
	deployment_id: Signal<Uuid>,
) -> impl IntoView {
	let deployment_info = get_deployment_query(deployment_id);

	let deployment_info_signal = create_rw_signal::<Option<GetDeploymentInfoResponse>>(None);
	provide_context(DeploymentInfoContext(deployment_info_signal));

	view! {
		<Transition>
			{move || match deployment_info.get() {
				Some(info) => {
					match info {
						Ok(data) => {
							let deployment = data.clone();
							deployment_info_signal.set(Some(deployment.clone()));
							view! {
								<ManageDeploymentHeader />
								<ContainerBody class="gap-md">
									<Outlet />
								</ContainerBody>
							}
								.into_view()
						}
						Err(err) => {
							view! {
								<ErrorPage
									title="Error Fetching Resource"
									content={view! {
										<p class="text-white">
											{format!("{}", err.to_string().to_case(Case::Title))}
										</p>
									}
										.into_view()}
								/>
							}
								.into_view()
						}
					}
				}
				None => view! { <div>"Loading"</div> }.into_view(),
			}}
		</Transition>
	}
}

/// A Wrapper around the manage deployments page content to make sure that
/// the runner id is a valid Uuid
#[component]
pub fn ManageDeployments() -> impl IntoView {
	let params = use_params::<DeploymentParams>();
	let deployment_id = Signal::derive(move || {
		params.with(|params| {
			params
				.as_ref()
				.map(|param| param.deployment_id.clone())
				.unwrap_or_default()
				.map(|x| Uuid::parse_str(x.as_str()).ok())
				.flatten()
		})
	});

	move || match deployment_id.get() {
		Some(deployment_id) => {
			view! { <ManageDeploymentsContent deployment_id={Signal::derive(move || deployment_id)} /> }
		}
		.into_view(),
		None => view! { <ErrorPage title="Deployment ID is not a valid UUID" /> }.into_view(),
	}
}
