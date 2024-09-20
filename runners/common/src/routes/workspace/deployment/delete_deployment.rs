use axum::http::StatusCode;
use models::api::workspace::{deployment::*, runner::StreamRunnerDataForWorkspaceServerMsg};

use crate::prelude::*;

/// The handler to delete a deployment in the workspace. This will delete the
/// deployment from the workspace, and remove all resources associated with the
/// deployment.
pub async fn delete_deployment(
	AppRequest {
		request:
			ProcessedApiRequest {
				path: DeleteDeploymentPath {
					workspace_id: _,
					deployment_id,
				},
				query: (),
				headers:
					DeleteDeploymentRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: DeleteDeploymentRequestProcessed,
			},
		database,
		config: _,
	}: AppRequest<'_, DeleteDeploymentRequest>,
) -> Result<AppResponse<DeleteDeploymentRequest>, ErrorType> {
	info!("Deleting deployment: {deployment_id}");

	query(
		r#"
		DELETE FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
	)
	.bind(deployment_id)
	.execute(&mut **database)
	.await?;

	trace!("Environment variables deleted");

	query(
		r#"
		DELETE FROM
			deployment_config_mounts
		WHERE
			deployment_id = $1;
		"#,
	)
	.bind(deployment_id)
	.execute(&mut **database)
	.await?;

	trace!("Config mounts deleted");

	// query(
	// 	r#"
	// 	DELETE FROM
	// 		deployment_deploy_history
	// 	WHERE
	// 		deployment_id = $1;
	// 	"#,
	// )
	// .bind(deployment_id)
	// .execute(&mut **database)
	// .await?;

	// trace!("Deploy history deleted");

	query(
		r#"
		DELETE FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
	)
	.bind(deployment_id)
	.execute(&mut **database)
	.await?;

	trace!("Exposed ports deleted");

	// Delete the deployment in the database
	query(
		r#"
		DELETE FROM
			deployment
		WHERE
			id = $1;
		"#,
	)
	.bind(deployment_id)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_foreign_key_violation() => ErrorType::ResourceInUse,
		err => ErrorType::server_error(err),
	})?;

	trace!("Deployment deleted");

	crate::runner::RUNNER_CHANGES_SENDER
		.get()
		.expect("Runner changes sender not set")
		.read()
		.expect("Runner changes sender lock poisoned")
		.send(Ok(
			StreamRunnerDataForWorkspaceServerMsg::DeploymentDeleted { id: deployment_id },
		))
		.expect("Failed to send deployment created message");

	trace!("Changes sent to runner");

	AppResponse::builder()
		.body(DeleteDeploymentResponse)
		.headers(())
		.status_code(StatusCode::RESET_CONTENT)
		.build()
		.into_result()
}
