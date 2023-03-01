use std::collections::BTreeMap;

use api_models::utils::Uuid;
use k8s_openapi::{
	api::{
		apps::v1::{Deployment, DeploymentSpec},
		core::v1::{
			Container,
			ContainerPort,
			EnvVar,
			LocalObjectReference,
			PodSpec,
			PodTemplateSpec,
			ResourceRequirements,
		},
	},
	apimachinery::pkg::{
		api::resource::Quantity,
		apis::meta::v1::LabelSelector,
	},
};
use kube::{
	api::{Patch, PatchParams},
	config::{
		AuthInfo,
		Cluster,
		Context,
		Kubeconfig,
		NamedAuthInfo,
		NamedCluster,
		NamedContext,
	},
	core::ObjectMeta,
	Api,
	Config,
};
use sqlx::Row;

use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	make_rbac_descriptions_non_nullable(&mut *connection, config).await?;
	add_secrets(&mut *connection, config).await?;

	update_roles_permissions(&mut *connection, config).await?;
	add_rbac_user_permissions(&mut *connection, config).await?;
	update_edit_workspace_permission(&mut *connection, config).await?;
	add_delete_workspace_permission(&mut *connection, config).await?;

	add_secret_id_column_to_deployment_environment_variable(
		&mut *connection,
		config,
	)
	.await?;

	remove_resource_requests_from_deployment(&mut *connection, config).await?;

	Ok(())
}

async fn make_rbac_descriptions_non_nullable(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		UPDATE resource_type
		SET description = '';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE resource_type
		ALTER COLUMN description SET NOT NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE role
		SET description = '';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE role
		ALTER COLUMN description SET NOT NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE permission
		SET description = '';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE permission
		ALTER COLUMN description SET NOT NULL;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn update_roles_permissions(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	for (old_permission, new_permission) in [
		("workspace::viewRoles", "workspace::rbac::role::list"),
		("workspace::createRole", "workspace::rbac::role::create"),
		("workspace::editRole", "workspace::rbac::role::edit"),
		("workspace::deleteRole", "workspace::rbac::role::delete"),
	] {
		query!(
			r#"
			UPDATE
				permission
			SET
				name = $1
			WHERE
				name = $2;
			"#,
			new_permission,
			old_permission,
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

async fn add_rbac_user_permissions(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	for &permission in [
		"workspace::rbac::user::list",
		"workspace::rbac::user::add",
		"workspace::rbac::user::remove",
		"workspace::rbac::user::updateRoles",
	]
	.iter()
	{
		let uuid = loop {
			let uuid = Uuid::new_v4();

			let exists = query!(
				r#"
				SELECT
					*
				FROM
					permission
				WHERE
					id = $1;
				"#,
				&uuid
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some();

			if !exists {
				// That particular resource ID doesn't exist. Use it
				break uuid;
			}
		};
		query!(
			r#"
			INSERT INTO
				permission
			VALUES
				($1, $2, $3);
			"#,
			&uuid,
			permission,
			"",
		)
		.execute(&mut *connection)
		.await?;
	}
	Ok(())
}

async fn update_edit_workspace_permission(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		UPDATE
			permission
		SET
			name = $1
		WHERE
			name = $2;
		"#,
		"workspace::edit",
		"workspace::editInfo",
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_delete_workspace_permission(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	let uuid = loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				permission
			WHERE
				id = $1;
			"#,
			&uuid
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			// That particular resource ID doesn't exist. Use it
			break uuid;
		}
	};

	query!(
		r#"
		INSERT INTO
			permission
		VALUES
			($1, $2, $3);
		"#,
		&uuid,
		"workspace::delete",
		"",
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_secrets(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE secret(
			id UUID CONSTRAINT secret_pk PRIMARY KEY,
			name CITEXT NOT NULL
				CONSTRAINT secret_chk_name_is_trimmed CHECK(name = TRIM(name)),
			workspace_id UUID NOT NULL,
			deployment_id UUID, /* For deployment specific secrets */
			CONSTRAINT secret_uq_workspace_id_name UNIQUE(workspace_id, name),
			CONSTRAINT secret_fk_id_workspace_id FOREIGN KEY(id, workspace_id)
				REFERENCES resource(id, owner_id),
			CONSTRAINT secret_fk_deployment_id_workspace_id
				FOREIGN KEY(deployment_id, workspace_id)
					REFERENCES deployment(id, workspace_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	for &permission in [
		"workspace::secret::list",
		"workspace::secret::create",
		"workspace::secret::edit",
		"workspace::secret::delete",
	]
	.iter()
	{
		let uuid = loop {
			let uuid = Uuid::new_v4();

			let exists = query!(
				r#"
				SELECT
					*
				FROM
					permission
				WHERE
					id = $1;
				"#,
				&uuid
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some();

			if !exists {
				// That particular resource ID doesn't exist. Use it
				break uuid;
			}
		};

		query!(
			r#"
			INSERT INTO
				permission
			VALUES
				($1, $2, '');
			"#,
			&uuid,
			permission
		)
		.execute(&mut *connection)
		.await?;
	}

	const SECRET: &str = "secret";
	// Insert new resource type into the database for secrets
	let (resource_type, uuid) = (
		SECRET,
		loop {
			let uuid = Uuid::new_v4();

			let exists = query!(
				r#"
				SELECT
					*
				FROM
					resource_type
				WHERE
					id = $1;
				"#,
				&uuid
			)
			.fetch_optional(&mut *connection)
			.await?
			.is_some();

			if !exists {
				// That particular resource ID doesn't exist. Use it
				break uuid;
			}
		},
	);

	query!(
		r#"
		INSERT INTO
			resource_type
		VALUES
			($1, $2, '');
		"#,
		&uuid,
		resource_type,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_secret_id_column_to_deployment_environment_variable(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE deployment_environment_variable
		ADD COLUMN secret_id UUID;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_environment_variable
		ALTER COLUMN value DROP NOT NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_environment_variable
		ADD CONSTRAINT deployment_environment_variable_fk_secret_id
		FOREIGN KEY(secret_id) REFERENCES secret(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_environment_variable
		ADD CONSTRAINT deployment_env_var_chk_value_secret_id_either_not_null
		CHECK(
			(
				value IS NOT NULL AND
				secret_id IS NULL
			) OR
			(
				value IS NULL AND
				secret_id IS NOT NULL
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn remove_resource_requests_from_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	let mut deployment_list = vec![];

	let db_items = query!(
		r#"
		SELECT
			deployment.id,
			deployment.name::TEXT as "name",
			deployment.workspace_id,
			workspace.name::TEXT as "workspace_name",
			docker_registry_repository.name::TEXT as "repository",
			deployment.image_tag,
			deployment.min_horizontal_scale,
			deployment.machine_type
		FROM
			deployment
		INNER JOIN
			workspace
		ON
			deployment.workspace_id = workspace.id
		INNER JOIN
			docker_registry_repository
		ON
			deployment.repository_id = docker_registry_repository.id
		WHERE
			deployment.status = 'running';
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| {
		(
			row.get::<Uuid, _>("id"),
			row.get::<String, _>("name"),
			row.get::<Uuid, _>("workspace_id"),
			row.get::<String, _>("workspace_name"),
			row.get::<String, _>("repository"),
			row.get::<String, _>("image_tag"),
			row.get::<i16, _>("min_horizontal_scale"),
			row.get::<Uuid, _>("machine_type"),
		)
	})
	.collect::<Vec<_>>();

	if db_items.is_empty() {
		return Ok(());
	}

	for (
		deployment_id,
		deployment_name,
		workspace_id,
		workspace_name,
		repository,
		image_tag,
		min_horizontal_scale,
		machine_type,
	) in db_items
	{
		let ports = query!(
			r#"
			SELECT
				port
			FROM
				deployment_exposed_port
			WHERE
				deployment_id = $1 AND
				port_type = 'http';
			"#,
			&deployment_id
		)
		.fetch_all(&mut *connection)
		.await?
		.into_iter()
		.map(|row| row.get::<i32, _>("port"))
		.collect::<Vec<_>>();

		let env_vars = query!(
			r#"
			SELECT
				name,
				value
			FROM
				deployment_environment_variable
			WHERE
				deployment_id = $1;
			"#,
			&deployment_id
		)
		.fetch_all(&mut *connection)
		.await?
		.into_iter()
		.map(|row| {
			(row.get::<String, _>("name"), row.get::<String, _>("value"))
		})
		.collect::<Vec<_>>();

		let (cpu, memory) = query!(
			r#"
			SELECT
				cpu_count,
				memory_count
			FROM
				deployment_machine_type
			WHERE
				id = $1;
			"#,
			machine_type
		)
		.fetch_one(&mut *connection)
		.await
		.map(|row| {
			(
				row.get::<i16, _>("cpu_count"),
				row.get::<i32, _>("memory_count"),
			)
		})?;

		let machine_type = [
			(
				"memory".to_string(),
				Quantity(format!("{:.1}G", (memory as f64) / 4f64)),
			),
			("cpu".to_string(), Quantity(format!("{:.1}", cpu as f64))),
		]
		.into_iter()
		.collect::<BTreeMap<_, _>>();

		deployment_list.push((
			deployment_id,
			deployment_name,
			workspace_id,
			workspace_name,
			repository,
			image_tag,
			min_horizontal_scale,
			ports,
			env_vars,
			machine_type,
		));
	}

	let kubernetes_config = Config::from_custom_kubeconfig(
		Kubeconfig {
			preferences: None,
			clusters: vec![NamedCluster {
				name: config.kubernetes.cluster_name.clone(),
				cluster: Some(Cluster {
					server: Some(config.kubernetes.cluster_url.clone()),
					insecure_skip_tls_verify: None,
					certificate_authority: None,
					certificate_authority_data: Some(
						config.kubernetes.certificate_authority_data.clone(),
					),
					proxy_url: None,
					extensions: None,
					..Default::default()
				}),
			}],
			auth_infos: vec![NamedAuthInfo {
				name: config.kubernetes.auth_name.clone(),
				auth_info: Some(AuthInfo {
					username: Some(config.kubernetes.auth_username.clone()),
					token: Some(config.kubernetes.auth_token.clone().into()),
					..Default::default()
				}),
			}],
			contexts: vec![NamedContext {
				name: config.kubernetes.context_name.clone(),
				context: Some(Context {
					cluster: config.kubernetes.cluster_name.clone(),
					user: config.kubernetes.auth_username.clone(),
					extensions: None,
					namespace: None,
				}),
			}],
			current_context: Some(config.kubernetes.context_name.clone()),
			extensions: None,
			kind: Some("Config".to_string()),
			api_version: Some("v1".to_string()),
		},
		&Default::default(),
	)
	.await
	.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;
	let client = kube::Client::try_from(kubernetes_config)
		.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;

	for (
		deployment_id,
		deployment_name,
		workspace_id,
		workspace_name,
		repository,
		image_tag,
		min_horizontal_scale,
		ports,
		env_vars,
		machine_type,
	) in deployment_list
	{
		let namespace = workspace_id.as_str();

		let deployment_api =
			Api::<Deployment>::namespaced(client.clone(), namespace);

		let labels = deployment_api
			.get(deployment_id.as_str())
			.await
			.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?
			.metadata
			.labels;

		let kubernetes_deployment = Deployment {
			metadata: ObjectMeta {
				name: Some(format!("deployment-{}", deployment_id)),
				namespace: Some(namespace.to_string()),
				labels: labels.clone(),
				..ObjectMeta::default()
			},
			spec: Some(DeploymentSpec {
				replicas: Some(min_horizontal_scale as i32),
				selector: LabelSelector {
					match_expressions: None,
					match_labels: labels.clone(),
				},
				template: PodTemplateSpec {
					spec: Some(PodSpec {
						containers: vec![Container {
							name: format!("deployment-{}", deployment_id),
							image: Some(format!(
								"registry.patr.cloud/{}/{}:{}",
								workspace_name, repository, image_tag
							)),
							image_pull_policy: Some("Always".to_string()),
							ports: Some(
								ports
									.into_iter()
									.map(|port| ContainerPort {
										container_port: port,
										..ContainerPort::default()
									})
									.collect::<Vec<_>>(),
							),
							env: Some(
								env_vars
									.into_iter()
									.map(|(name, value)| EnvVar {
										name,
										value: Some(value),
										..EnvVar::default()
									})
									.chain([
										EnvVar {
											name: "PATR".to_string(),
											value: Some("true".to_string()),
											..EnvVar::default()
										},
										EnvVar {
											name: "WORKSPACE_ID".to_string(),
											value: Some(
												workspace_id.to_string(),
											),
											..EnvVar::default()
										},
										EnvVar {
											name: "DEPLOYMENT_ID".to_string(),
											value: Some(
												deployment_id.to_string(),
											),
											..EnvVar::default()
										},
										EnvVar {
											name: "DEPLOYMENT_NAME".to_string(),
											value: Some(
												deployment_name.clone(),
											),
											..EnvVar::default()
										},
									])
									.collect::<Vec<_>>(),
							),
							resources: Some(ResourceRequirements {
								limits: Some(machine_type.clone()),
								..ResourceRequirements::default()
							}),
							..Container::default()
						}],
						image_pull_secrets: Some(vec![LocalObjectReference {
							name: Some("patr-regcred".to_string()),
						}]),
						..PodSpec::default()
					}),
					metadata: Some(ObjectMeta {
						labels,
						..ObjectMeta::default()
					}),
				},
				..DeploymentSpec::default()
			}),
			..Deployment::default()
		};

		deployment_api
			.patch(
				&format!("deployment-{}", deployment_id),
				&PatchParams::apply(&format!("deployment-{}", deployment_id)),
				&Patch::Apply(kubernetes_deployment),
			)
			.await
			.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;
	}

	Ok(())
}
