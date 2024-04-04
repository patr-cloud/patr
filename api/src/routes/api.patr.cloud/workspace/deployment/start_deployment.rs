use std::{cmp::Ordering, collections::BTreeMap};

use axum::{http::StatusCode, Router};
use futures::sink::With;
use models::{
	api::{
		workspace::{
			container_registry::{ContainerRepository, ContainerRepositoryTagInfo},
			infrastructure::{
				deployment::*,
				managed_url::{DbManagedUrlType, ManagedUrl, ManagedUrlType},
			},
			region::{Region, RegionStatus},
		},
		WithId,
	},
	utils::StringifiedU16,
	ApiRequest,
	ErrorType,
};
use sqlx::query_as;
use time::OffsetDateTime;

use crate::{models::deployment::MACHINE_TYPES, prelude::*, utils::validator};

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
			region
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
		(registry, deployment.image_tag, deployment.region)
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
				.await
				.map(|_| ());
			}
		}
	}

	let (image_name, digest) = match registry {
		DeploymentRegistry::PatrRegistry {
			registry: _,
			repository_id,
		} => {
			let digest = query!(
				r#"
				SELECT
					manifest_digest
				FROM
					container_registry_repository_tag
				WHERE
					repository_id = $1 AND
					tag = $2;
				"#,
				repository_id as _,
				image_tag
			)
			.fetch_optional(&mut **database)
			.await?
			.map(|row| row.manifest_digest);

			let repository_name = query!(
				r#"
				SELECT
					name
				FROM
					container_registry_repository
				WHERE
					id = $1 AND
					deleted IS NULL;
				"#,
				repository_id as _
			)
			.fetch_optional(&mut **database)
			.await?
			.map(|repo| repo.name)
			.ok_or(ErrorType::ResourceDoesNotExist)?;

			if let Some(digest) = digest {
				Ok((
					format!(
						"{}/{}/{}",
						todo!("config.docker_registry.registry_url"),
						workspace_id,
						repository_name
					),
					Some(digest),
				))
			} else {
				Ok((
					format!(
						"{}/{}/{}:{}",
						todo!("config.docker_registry.registry_url"),
						workspace_id,
						repository_name,
						image_tag
					),
					None,
				))
			}
		}
		DeploymentRegistry::ExternalRegistry {
			registry,
			image_name,
		} => match registry.as_str() {
			"registry.hub.docker.com" | "hub.docker.com" | "index.docker.io" | "docker.io" | "" => {
				Ok((format!("docker.io/{}:{}", image_name, image_tag), None))
			}
			_ => Ok((format!("{}/{}:{}", registry, image_name, image_tag), None)),
		},
	}?;

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

	let volumes = query!(
		r#"
		SELECT
			id,
			name,
			volume_size,
			volume_mount_path
		FROM
			deployment_volume
		WHERE
			deployment_id = $1 AND
			deleted IS NULL;
		"#,
		deployment_id as _,
	)
	.fetch_all(&mut **database)
	.await?
	.into_iter()
	.map(|volume| {
		(
			volume.name,
			DeploymentVolume {
				path: volume.volume_mount_path,
				size: volume.volume_size as u16,
			},
		)
	})
	.collect();

	if todo!("If deployment on patr region") {
		for volume in &volumes {
			todo!("Start usage history for volume")
		}
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
