use http::StatusCode;
use models::{api::workspace::deployment::*, prelude::*};

use crate::prelude::*;

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
		let cpu_count = machine.try_get::<i32, _>("cpu_count")?;
		let memory_count = machine.try_get::<i32, _>("memory_count")?;

		Ok(WithId::new(
			id,
			DeploymentMachineType {
				cpu_count: cpu_count as u16,
				memory_count: memory_count as u32,
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
