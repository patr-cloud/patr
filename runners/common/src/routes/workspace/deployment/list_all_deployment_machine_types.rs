use http::StatusCode;
use models::{api::workspace::deployment::*, prelude::*};

use crate::prelude::*;

/// List all deployment machine types. This is a public endpoint. No
/// authentication is required. This endpoint is used to list all the machine
/// types that are available for deployments.
pub async fn list_all_deployment_machine_types(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: ListAllDeploymentMachineTypePath { workspace_id: _ },
				query: (),
				headers: ListAllDeploymentMachineTypeRequestHeaders { user_agent: _ },
				body: ListAllDeploymentMachineTypeRequestProcessed,
			},
		database,
		runner_changes_sender: _,
		config: _,
	}: AppRequest<'_, ListAllDeploymentMachineTypeRequest>,
) -> Result<AppResponse<ListAllDeploymentMachineTypeRequest>, ErrorType> {
	info!("Listing all Deployment Machine Types");

	let machine_types = query(
		r#"
        SELECT
            id,
            cpu_count,
            memory_count
        FROM
            deployment_machine_type;
        "#,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|machine| {
		let id = machine.try_get::<Uuid, _>("id")?;
		let cpu_count = machine.try_get::<u16, _>("cpu_count")?;
		let memory_count = machine.try_get::<u32, _>("memory_count")?;

		Ok(WithId::new(
			id,
			DeploymentMachineType {
				cpu_count,
				memory_count,
			},
		))
	})
	.collect::<Result<_, ErrorType>>()?;

	AppResponse::builder()
		.body(ListAllDeploymentMachineTypeResponse { machine_types })
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
