use axum::http::StatusCode;
use models::api::workspace::deployment::*;

use crate::prelude::*;

/// The handler to delete a deployment in the workspace. This will delete the
/// deployment from the workspace, and remove all resources associated with the
/// deployment.
pub async fn delete_deployment(
	AppRequest {
		config: _,
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

	query(
		r#"
		DELETE FROM
			deployment_deploy_history
		WHERE
			deployment_id = $1;
		"#,
	)
	.bind(deployment_id)
	.execute(&mut **database)
	.await?;

	query("PRAGMA defer_foreign_keys = ON;")
		.execute(&mut **database)
		.await?;

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

	query("PRAGMA defer_foreign_keys = OFF;")
		.execute(&mut **database)
		.await?;

	AppResponse::builder()
		.body(DeleteDeploymentResponse)
		.headers(())
		.status_code(StatusCode::RESET_CONTENT)
		.build()
		.into_result()
}
