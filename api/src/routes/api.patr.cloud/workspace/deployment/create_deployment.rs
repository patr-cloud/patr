use axum::http::StatusCode;
use cloudflare::{
	endpoints::workerskv::write_key,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
use models::{
	api::workspace::{deployment::*, runner::StreamRunnerDataForWorkspaceServerMsg},
	cloudflare::kv::*,
};
use rustis::commands::PubSubCommands;
use time::OffsetDateTime;

use crate::prelude::*;

/// The handler to create a deployment in the workspace. This will create a new
/// deployment in the workspace, and return the ID of the deployment.
pub async fn create_deployment(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path: CreateDeploymentPath { workspace_id },
				query: (),
				headers:
					CreateDeploymentRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body:
					CreateDeploymentRequestProcessed {
						name,
						registry,
						image_tag,
						runner,
						machine_type,
						running_details:
							DeploymentRunningDetails {
								deploy_on_push,
								min_horizontal_scale,
								max_horizontal_scale,
								ports,
								environment_variables,
								startup_probe,
								liveness_probe,
								config_mounts,
							},
						deploy_on_create,
					},
			},
		database,
		redis,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, CreateDeploymentRequest>,
) -> Result<AppResponse<CreateDeploymentRequest>, ErrorType> {
	info!("Creating deployment with name `{name}` in workspace: {workspace_id}");

	let now = OffsetDateTime::now_utc();

	let deployment_id = query!(
		r#"
		INSERT INTO
			resource(
				id,
				resource_type_id,
				owner_id,
				created,
				deleted
			)
		VALUES
			(
				GENERATE_RESOURCE_ID(),
				(SELECT id FROM resource_type WHERE name = 'deployment'),
				$1,
				$2,
				NULL
			)
		RETURNING id AS "id: Uuid";
		"#,
		workspace_id as _,
		now as _,
	)
	.fetch_one(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		err => ErrorType::server_error(err),
	})?
	.id;

	// BEGIN DEFERRED CONSTRAINT
	query!(
		r#"
		SET CONSTRAINTS ALL DEFERRED;
		"#,
	)
	.execute(&mut **database)
	.await?;

	query!(
		r#"
		INSERT INTO
			deployment(
				id,
				name,
				registry,
				repository_id,
				image_name,
				image_tag,
				status,
				workspace_id,
				runner,
				min_horizontal_scale,
				max_horizontal_scale,
				machine_type,
				deploy_on_push,
				startup_probe_port,
				startup_probe_path,
				startup_probe_port_type,
				liveness_probe_port,
				liveness_probe_path,
				liveness_probe_port_type
			)
		VALUES
			(
				$1,
				$2,
				$3,
				$4,
				$5,
				$6,
				$7,
				$8,
				$9,
				$10,
				$11,
				$12,
				$13,
				$14,
				$15,
				$16,
				$17,
				$18,
				$19
			);
		"#,
		deployment_id as _,
		name as _,
		registry.registry_url(),
		registry.repository_id() as _,
		registry.image_name(),
		image_tag.as_ref(),
		if deploy_on_create {
			DeploymentStatus::Deploying
		} else {
			DeploymentStatus::Stopped
		} as _,
		workspace_id as _,
		runner as _,
		min_horizontal_scale as i32,
		max_horizontal_scale as i32,
		machine_type as _,
		deploy_on_push,
		startup_probe.as_ref().map(|probe| probe.port as i32),
		startup_probe.as_ref().map(|probe| probe.path.as_str()),
		startup_probe.as_ref().map(|_| ExposedPortType::Http) as _,
		liveness_probe.as_ref().map(|probe| probe.port as i32),
		liveness_probe.as_ref().map(|probe| probe.path.as_str()),
		liveness_probe.as_ref().map(|_| ExposedPortType::Http) as _,
	)
	.execute(&mut **database)
	.await
	.map_err(|err| match err {
		sqlx::Error::Database(err) if err.is_unique_violation() => ErrorType::ResourceAlreadyExists,
		err => ErrorType::server_error(err),
	})?;

	trace!("Created deployment with ID: {}", deployment_id);

	query!(
		r#"
		INSERT INTO 
			deployment_exposed_port(
				deployment_id,
				port,
				port_type
			)
		SELECT
			*
		FROM
			UNNEST(
				$1::UUID[],
				$2::INTEGER[],
				$3::EXPOSED_PORT_TYPE[]
			);
		"#,
		&ports
			.iter()
			.map(|_| deployment_id.into())
			.collect::<Vec<_>>(),
		&ports
			.iter()
			.map(|(port, _)| port.value() as i32)
			.collect::<Vec<_>>(),
		&ports
			.iter()
			.map(|(_, port_type)| port_type.to_string())
			.collect::<Vec<String>>() as _,
	)
	.execute(&mut **database)
	.await?;

	trace!("Inserted exposed ports for deployment");

	// END DEFERRED CONSTRAINT
	query!(
		r#"
		SET CONSTRAINTS ALL IMMEDIATE;
		"#,
	)
	.execute(&mut **database)
	.await?;

	trace!("Set constraints to immediate");

	query!(
		r#"
		INSERT INTO
			deployment_environment_variable(
				deployment_id,
				name,
				value
			)
		SELECT
			*
		FROM
			UNNEST(
				$1::UUID[],
				$2::TEXT[],
				$3::TEXT[]
			);
		"#,
		&environment_variables
			.iter()
			.map(|_| deployment_id.into())
			.collect::<Vec<_>>(),
		&environment_variables
			.iter()
			.map(|(name, _)| name.clone())
			.collect::<Vec<_>>(),
		&environment_variables
			.iter()
			.map(|(_, value)| value.clone())
			.collect::<Vec<String>>() as _,
	)
	.execute(&mut **database)
	.await?;

	trace!("Inserted environment variables for deployment");

	query!(
		r#"
		INSERT INTO 
			deployment_config_mounts(
				deployment_id,
				path,
				file
			)
		SELECT
			*
		FROM
			UNNEST(
				$1::UUID[],
				$2::TEXT[],
				$3::BYTEA[]
			);
		"#,
		&config_mounts
			.iter()
			.map(|_| deployment_id.into())
			.collect::<Vec<_>>(),
		&config_mounts
			.iter()
			.map(|(path, _)| path.clone())
			.collect::<Vec<_>>(),
		&config_mounts
			.iter()
			.map(|(_, file)| file.to_vec())
			.collect::<Vec<_>>(),
	)
	.execute(&mut **database)
	.await?;

	if let DeploymentRegistry::PatrRegistry { repository_id, .. } = &registry {
		let digest = query!(
			r#"
			SELECT
				manifest_digest
			FROM
				container_registry_repository_tag
			WHERE
				repository_id = $1 AND
				name = $2;
			"#,
			repository_id as _,
			image_tag as _
		)
		.fetch_optional(&mut **database)
		.await?
		.map(|row| row.manifest_digest);

		if let Some(digest) = digest {
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

	CloudflareClient::new(
		Credentials::UserAuthToken {
			token: state.config.cloudflare.api_key.clone(),
		},
		ClientConfig::default(),
		Environment::Custom(state.config.cloudflare.base_url.clone()),
	)?
	.request(&write_key::WriteKey {
		account_identifier: &state.config.cloudflare.account_id,
		namespace_identifier: &state.config.cloudflare.worker_namespace_id,
		key: &deployment_id.to_string(),
		params: write_key::WriteKeyParams {
			expiration: None,
			expiration_ttl: None,
		},
		body: write_key::WriteKeyBody::Value(serde_json::to_vec(&InternalKVData::Deployment {
			ports: ports.keys().map(|port| port.value()).collect(),
			runner_id: runner,
			status: DeploymentStatus::Deploying,
		})?),
	})
	.await?;

	// TODO Temporary workaround until audit logs and triggers are implemented
	redis
		.publish(
			format!("{}/runner/{}/stream", workspace_id, runner),
			serde_json::to_string(&StreamRunnerDataForWorkspaceServerMsg::DeploymentCreated {
				deployment: WithId::new(
					deployment_id,
					Deployment {
						name: name.to_string(),
						registry,
						image_tag: image_tag.to_string(),
						runner,
						status: DeploymentStatus::Deploying,
						current_live_digest: None,
						machine_type,
					},
				),
				running_details: DeploymentRunningDetails {
					deploy_on_push,
					min_horizontal_scale,
					max_horizontal_scale,
					ports,
					environment_variables,
					startup_probe,
					liveness_probe,
					config_mounts,
				},
			})
			.unwrap(),
		)
		.await?;

	AppResponse::builder()
		.body(CreateDeploymentResponse {
			id: WithId::from(deployment_id),
		})
		.headers(())
		.status_code(StatusCode::CREATED)
		.build()
		.into_result()
}
