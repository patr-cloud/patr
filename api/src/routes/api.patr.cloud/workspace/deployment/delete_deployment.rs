use axum::http::StatusCode;
use models::{api::workspace::deployment::*, ErrorType};

use crate::prelude::*;

/// The handler to delete a deployment in the workspace. This will delete the
/// deployment from the workspace, and remove all resources associated with the
/// deployment.
pub async fn delete_deployment(
	AuthenticatedAppRequest {
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
		redis: _,
		client_ip: _,
		config: _,
		user_data: _,
	}: AuthenticatedAppRequest<'_, DeleteDeploymentRequest>,
) -> Result<AppResponse<DeleteDeploymentRequest>, ErrorType> {
	info!("Deleting deployment: {deployment_id}");

	query!(
		r#"
		DELETE FROM
			deployment_environment_variable
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		DELETE FROM
			deployment_config_mounts
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		DELETE FROM
			deployment_deploy_history
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		SET CONSTRAINTS ALL DEFERRED;
		"#
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		DELETE FROM
			deployment_exposed_port
		WHERE
			deployment_id = $1;
		"#,
		deployment_id as _
	)
	.execute(&mut **database)
	.await?;

	// Mark deployment deleted in database
	query!(
		r#"
		DELETE FROM
			deployment
		WHERE
			id = $1;
		"#,
		deployment_id as _
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_foreign_key_violation() => ErrorType::ResourceInUse,
		_ => ErrorType::InternalServerError,
	})?;

	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#
	)
	.execute(&mut **database)
	.await?;

	AppResponse::builder()
		.body(DeleteDeploymentResponse)
		.headers(())
		.status_code(StatusCode::RESET_CONTENT)
		.build()
		.into_result()
}
