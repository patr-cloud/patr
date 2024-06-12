use std::{cmp::Ordering, collections::BTreeMap};

use axum::{http::StatusCode, Router};
use futures::sink::With;
use models::{api::workspace::deployment::*, ErrorType};
use sqlx::query_as;
use time::OffsetDateTime;

use crate::prelude::*;

/// Start deployment
///
/// #Parameters
/// - `workspace_id`: The workspace ID
/// - `deployment_id`: The deployment ID
///
/// #Returns
/// - `OK`: The deployment was started
pub async fn start_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: StartDeploymentPath {
					workspace_id,
					deployment_id,
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
	}: AuthenticatedAppRequest<'_, StartDeploymentRequest>,
) -> Result<AppResponse<StartDeploymentRequest>, ErrorType> {
	info!("Starting: Start deployment");

	let now = OffsetDateTime::now_utc();

	let (registry, image_tag, region) = query!(
		r#"
		SELECT
			registry,
			repository_id,
			image_name,
			image_tag,
			runner
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
	.map(|deployment| {
		let registry = if deployment.registry == PatrRegistry.to_string() {
			DeploymentRegistry::PatrRegistry {
				registry: PatrRegistry,
				repository_id: deployment.repository_id.unwrap().into(),
			}
		} else {
			DeploymentRegistry::ExternalRegistry {
				registry: deployment.registry,
				image_name: deployment.image_name.unwrap().into(),
			}
		};
		(registry, deployment.image_tag, deployment.runner)
	})
	.ok_or(ErrorType::ResourceDoesNotExist)?;

	if let DeploymentRegistry::PatrRegistry { repository_id, .. } = &registry {
		let digest = query!(
			r#"
			SELECT
				manifest_digest
			FROM
				container_registry_repository_manifest
			WHERE
				repository_id = $1
			ORDER BY
				created DESC
			LIMIT 1;
			"#,
			repository_id as _
		)
		.fetch_optional(&mut **database)
		.await?
		.map(|row| row.manifest_digest);

		if let Some(digest) = digest {
			// Check if digest is already in deployment_deploy_history table
			let deployment_deploy_history = query_as!(
				DeploymentDeployHistory,
				r#"
				SELECT 
					image_digest,
					created as "created: _"
				FROM
					deployment_deploy_history
				WHERE
					image_digest = $1;
				"#,
				digest as _,
			)
			.fetch_optional(&mut **database)
			.await?;

			// If not, add it to the table
			if deployment_deploy_history.is_none() {
				query!(
					r#"
					INSERT INTO
						deployment_deploy_history(
							deployment_id,
							image_digest,
							repository_id,
							created
						)
					VALUES
						($1, $2, $3, $4)
					ON CONFLICT
						(deployment_id, image_digest)
					DO NOTHING;
					"#,
					deployment_id as _,
					digest as _,
					repository_id as _,
					now as _,
				)
				.execute(&mut **database)
				.await?;
			}
		}
	}

	// Update status to deploying
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

	if todo!("If deployment on patr region") {
		todo!("Start usage history for volume");
		todo!("Start usage history for deployment");
	}

	todo!("Audit log");

	AppResponse::builder()
		.body(StartDeploymentResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
