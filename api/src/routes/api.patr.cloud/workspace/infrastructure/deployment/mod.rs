mod create_deployment;
mod delete_deployment;
mod get_deployment_info;
mod get_deployment_log;
mod get_deployment_metric;
mod list_deployment_history;
mod list_deployment;
mod list_linked_url;
mod revert_deployment;
mod start_deployment;
mod stop_deployment;
mod update_deployment;

use axum::{http::StatusCode, Router};
use models::api::{workspace::infrastructure::deployment::{
		DeploymentMachineType, 
		ListAllDeploymentMachineTypeRequest, 
		ListAllDeploymentMachineTypeResponse
	}, 
	WithId
};
use crate::prelude::*;

use self::{
	create_deployment::*,
	delete_deployment::*,
	get_deployment_info::*,
	get_deployment_log::*,
	get_deployment_metric::*,
	list_deployment_history::*,
	list_deployment::*,
	list_linked_url::*,
	revert_deployment::*,
	start_deployment::*,
	stop_deployment::*,
	update_deployment::*,
};

#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_endpoint(machine_type, state)
		.mount_auth_endpoint(list_deployment, state)
		.mount_auth_endpoint(list_deployment_history, state)
		.mount_auth_endpoint(create_deployment, state)
		.mount_auth_endpoint(get_deployment_info, state)
		.mount_auth_endpoint(start_deployment, state)
		.mount_auth_endpoint(stop_deployment, state)
		.mount_auth_endpoint(revert_deployment, state)
		.mount_auth_endpoint(get_deployment_log, state)
		.mount_auth_endpoint(delete_deployment, state)
		.mount_auth_endpoint(update_deployment, state)
		.mount_auth_endpoint(list_linked_url, state)
		.mount_auth_endpoint(get_deployment_metric, state)
		.with_state(state.clone())
}

async fn machine_type(
	AppRequest {
		request: ProcessedApiRequest {
			path,
			query: _,
			headers,
			body,
		},
		database,
		redis: _,
		client_ip: _,
		config,
	}: AppRequest<'_, ListAllDeploymentMachineTypeRequest>,
) -> Result<AppResponse<ListAllDeploymentMachineTypeRequest>, ErrorType> {
	info!("Starting: List deployments");

	let machine_types = query!(
		r#"
		SELECT
			id,
			cpu_count,
			memory_count
		FROM
			deployment_machine_type;
		"#
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|machine| {
		WithId::new(
			machine.id,
			DeploymentMachineType {
				cpu_count: machine.cpu_count,
				memory_count: machine.memory_count,
			},
		)
	})
	.collect();

	AppResponse::builder()
		.body(ListAllDeploymentMachineTypeResponse { machine_types })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}