use axum::http::StatusCode;
use models::api::workspace::deployment::*;
use time::OffsetDateTime;

use crate::prelude::*;

/// The handler to start a deployment in the workspace. This will start
/// the deployment. In case the deployment is already running, it will
/// do nothing.
pub async fn start_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: StartDeploymentPath {
					workspace_id: _,
					deployment_id,
				},
				query: StartDeploymentQuery { force_restart },
				headers:
					StartDeploymentRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: StartDeploymentRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		config,
		user_data,
	}: AuthenticatedAppRequest<'_, StartDeploymentRequest>,
) -> Result<AppResponse<StartDeploymentRequest>, ErrorType> {
	info!("Starting deployment: {}", deployment_id);

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
			deleted IS NULL;
		"#,
		deployment_id as _
	)
	.fetch_optional(&mut **database)
	.await?
	.and_then(|deployment| {
		let registry = if deployment.registry == PatrRegistry.to_string() {
			DeploymentRegistry::PatrRegistry {
				registry: PatrRegistry,
				repository_id: deployment.repository_id?.into(),
			}
		} else {
			DeploymentRegistry::ExternalRegistry {
				registry: deployment.registry,
				image_name: deployment.image_name?,
			}
		};
		Some((registry, deployment.image_tag, deployment.runner))
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
			let deployment_deploy_history = query!(
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

	AppResponse::builder()
		.body(StartDeploymentResponse)
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
