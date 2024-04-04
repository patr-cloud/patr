use std::{cmp::Ordering, collections::BTreeMap};

use axum::{http::StatusCode, Router};
use futures::sink::With;
use models::{
	api::workspace::infrastructure::deployment::*,
	ErrorType,
};
use sqlx::query_as;
use time::OffsetDateTime;

use crate::prelude::*;

pub async fn revert_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					RevertDeploymentPath {
						workspace_id,
						deployment_id,
						digest,
					},
				query: _,
				headers,
				body,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, RevertDeploymentRequest>,
) -> Result<AppResponse<RevertDeploymentRequest>, ErrorType> {
	info!("Starting: Revert deployment");

	// Check if deployment exists
	query!(
		r#"
		SELECT
			id
		FROM
			deployment
		WHERE
			id = $1 AND
			status != 'deleted';
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	// Check if the digest is present or not in the deployment_deploy_history
	// table
	query!(
		r#"
		SELECT 
			image_digest,
			created
		FROM
			deployment_deploy_history
		WHERE
			image_digest = $1;
		"#,
		digest as _,
	)
	.fetch_optional(&mut **database)
	.await?
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	// Revert the digest
	query!(
		r#"
		UPDATE
			deployment
		SET
			current_live_digest = $1
		WHERE
			id = $2;
		"#,
		digest as _,
		deployment_id as _
	)
	.execute(&mut **database)
	.await?;

	// Set deployment status to deploying
	query!(
		r#"
		UPDATE
			deployment
		SET
			status = $1
		WHERE
			id = $2;
		"#,
		DeploymentStatus::Deploying as _,
		deployment_id as _
	)
	.execute(&mut **database)
	.await?;

	todo!("Audit log");

	AppResponse::builder()
		.body(RevertDeploymentResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}