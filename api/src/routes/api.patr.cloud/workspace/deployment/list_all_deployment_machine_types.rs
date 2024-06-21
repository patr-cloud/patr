use axum::http::StatusCode;
use models::api::workspace::deployment::*;

use crate::prelude::*;

/// List all deployment machine types. This is a public endpoint. No
/// authentication is required. This endpoint is used to list all the machine
/// types that are available for deployments.
pub async fn machine_type(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: ListAllDeploymentMachineTypePath { workspace_id: _ },
				query: (),
				headers: ListAllDeploymentMachineTypeRequestHeaders { user_agent: _ },
				body: ListAllDeploymentMachineTypeRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config: _,
	}: AppRequest<'_, ListAllDeploymentMachineTypeRequest>,
) -> Result<AppResponse<ListAllDeploymentMachineTypeRequest>, ErrorType> {
	info!("Listing all deployment machine types");

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
				cpu_count: machine.cpu_count as u16,
				memory_count: machine.memory_count as u32,
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
