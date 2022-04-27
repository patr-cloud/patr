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
use reqwest::Client;
use sqlx::Row;

use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	migrate_from_docr_to_pcr(connection, config).await?;
	audit_logs(connection, config).await?;
	rename_backup_email_to_recovery_email(connection, config).await?;
	chargebee(connection, config).await?;
	remove_domain_tlds_in_ci(connection, config).await?;

	Ok(())
}

async fn remove_domain_tlds_in_ci(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	let users = query!(
		r#"
		SELECT
			COUNT(*) as "users"
		FROM
			"user";
		"#
	)
	.fetch_one(&mut *connection)
	.await?
	.get::<i64, _>("users");

	if users == 0 {
		query!(
			r#"
			DELETE FROM domain_tld;
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

async fn migrate_from_docr_to_pcr(
	connection: &mut sqlx::PgConnection,
	config: &Settings,
) -> Result<(), Error> {
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
			deployment.region,
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
			row.get::<Uuid, _>("region"),
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
		region,
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
			region,
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
				cluster: Cluster {
					server: config.kubernetes.cluster_url.clone(),
					insecure_skip_tls_verify: None,
					certificate_authority: None,
					certificate_authority_data: Some(
						config.kubernetes.certificate_authority_data.clone(),
					),
					proxy_url: None,
					extensions: None,
				},
			}],
			auth_infos: vec![NamedAuthInfo {
				name: config.kubernetes.auth_name.clone(),
				auth_info: AuthInfo {
					username: Some(config.kubernetes.auth_username.clone()),
					token: Some(config.kubernetes.auth_token.clone().into()),
					..Default::default()
				},
			}],
			contexts: vec![NamedContext {
				name: config.kubernetes.context_name.clone(),
				context: Context {
					cluster: config.kubernetes.cluster_name.clone(),
					user: config.kubernetes.auth_username.clone(),
					extensions: None,
					namespace: None,
				},
			}],
			current_context: Some(config.kubernetes.context_name.clone()),
			extensions: None,
			kind: Some("Config".to_string()),
			api_version: Some("v1".to_string()),
		},
		&Default::default(),
	)
	.await?;
	let client = kube::Client::try_from(kubernetes_config)?;

	for (
		deployment_id,
		deployment_name,
		workspace_id,
		workspace_name,
		repository,
		image_tag,
		region,
		min_horizontal_scale,
		ports,
		env_vars,
		machine_type,
	) in deployment_list
	{
		let namespace = workspace_id.as_str();

		let labels = [
			("deploymentId".to_string(), deployment_id.to_string()),
			("workspaceId".to_string(), workspace_id.to_string()),
			("region".to_string(), region.to_string()),
		]
		.into_iter()
		.collect::<BTreeMap<_, _>>();

		let kubernetes_deployment = Deployment {
			metadata: ObjectMeta {
				name: Some(format!("deployment-{}", deployment_id)),
				namespace: Some(namespace.to_string()),
				labels: Some(labels.clone()),
				..ObjectMeta::default()
			},
			spec: Some(DeploymentSpec {
				replicas: Some(min_horizontal_scale as i32),
				selector: LabelSelector {
					match_expressions: None,
					match_labels: Some(labels.clone()),
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
								requests: Some(machine_type),
							}),
							..Container::default()
						}],
						image_pull_secrets: Some(vec![LocalObjectReference {
							name: Some("patr-regcred".to_string()),
						}]),
						..PodSpec::default()
					}),
					metadata: Some(ObjectMeta {
						labels: Some(labels.clone()),
						..ObjectMeta::default()
					}),
				},
				..DeploymentSpec::default()
			}),
			..Deployment::default()
		};

		let deployment_api =
			Api::<Deployment>::namespaced(client.clone(), namespace);

		deployment_api
			.patch(
				&format!("deployment-{}", deployment_id),
				&PatchParams::apply(&format!("deployment-{}", deployment_id)),
				&Patch::Apply(kubernetes_deployment),
			)
			.await?;
	}
	Ok(())
}

async fn audit_logs(
	connection: &mut sqlx::PgConnection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE workspace_audit_log (
			id UUID NOT NULL CONSTRAINT workspace_audit_log_pk PRIMARY KEY,
			date TIMESTAMPTZ NOT NULL,
			ip_address TEXT NOT NULL,
			workspace_id UUID NOT NULL
				CONSTRAINT workspace_audit_log_fk_workspace_id
					REFERENCES workspace(id),
			user_id UUID,
			login_id UUID,
			resource_id UUID NOT NULL,
			action UUID NOT NULL,
			request_id UUID NOT NULL,
			metadata JSON NOT NULL,
			patr_action BOOL NOT NULL,
			success BOOL NOT NULL,
			CONSTRAINT workspace_audit_log_chk_patr_action CHECK(
				(
					patr_action = true AND
					user_id IS NULL AND
					login_id IS NULL
				) OR
				(
					patr_action = false AND
					user_id IS NOT NULL AND
					login_id IS NOT NULL
				)
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_audit_log
			ADD CONSTRAINT workspace_audit_log_fk_user_id
				FOREIGN KEY(user_id) REFERENCES "user"(id),
			ADD CONSTRAINT workspace_audit_log_fk_login_id
				FOREIGN KEY(user_id, login_id) REFERENCES user_login(user_id, login_id),
			ADD CONSTRAINT workspace_audit_log_fk_resource_id
				FOREIGN KEY(resource_id) REFERENCES resource(id),
			ADD CONSTRAINT workspace_audit_log_fk_action
				FOREIGN KEY(action) REFERENCES permission(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn chargebee(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	let workspaces = query!(
		r#"
		SELECT
			id,
			super_admin_id
		FROM
			workspace;
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| {
		(
			row.get::<Uuid, _>("id"),
			row.get::<Uuid, _>("super_admin_id"),
		)
	})
	.collect::<Vec<_>>();

	if workspaces.is_empty() {
		return Ok(());
	}

	for (workspace_id, user_id) in workspaces {
		let user_data = query!(
			r#"
			SELECT
				first_name,
				last_name
			FROM
				"user"
			WHERE
				id = $1;
			"#,
			user_id
		)
		.fetch_one(&mut *connection)
		.await?;

		let (first_name, last_name) = (
			user_data.get::<String, _>("first_name"),
			user_data.get::<String, _>("last_name"),
		);

		let client = Client::new();

		let password: Option<String> = None;

		client
			.post(format!("{}/customers", config.chargebee.url))
			.basic_auth(config.chargebee.api_key.as_str(), password.as_ref())
			.query(&[
				("first_name", first_name),
				("last_name", last_name),
				("id", workspace_id.to_string()),
			])
			.send()
			.await?;

		client
			.post(format!("{}/promotional_credits/set", config.chargebee.url))
			.basic_auth(config.chargebee.api_key.as_str(), password.as_ref())
			.query(&[
				("customer_id", workspace_id.as_str()),
				("amount", &config.chargebee.credit_amount),
				("description", &config.chargebee.description),
			])
			.send()
			.await?;

		let deployments = query!(
			r#"
			SELECT
				id,
				min_horizontal_scale,
				machine_type
			FROM
				deployment
			WHERE
				workspace_id = $1 AND
				status != 'deleted';
			"#,
			&workspace_id
		)
		.fetch_all(&mut *connection)
		.await?
		.into_iter()
		.map(|row| {
			(
				row.get::<Uuid, _>("id"),
				row.get::<i16, _>("min_horizontal_scale"),
				row.get::<Uuid, _>("machine_type"),
			)
		})
		.collect::<Vec<_>>();

		for (deployment_id, min_horizontal_scale, machine_type) in deployments {
			let client = Client::new();

			let password: Option<String> = None;

			client
				.post(format!(
					"{}/customers/{}/subscription_for_items",
					config.chargebee.url, workspace_id
				))
				.basic_auth(&config.chargebee.api_key, password)
				.query(&[
					("id", deployment_id.to_string()),
					(
						"subscription_items[item_price_id][0]",
						format!("{}-USD-Monthly", machine_type),
					),
					(
						"subscription_items[quantity][0]",
						min_horizontal_scale.to_string(),
					),
				])
				.send()
				.await?;
		}
	}

	Ok(())
}

async fn rename_backup_email_to_recovery_email(
	connection: &mut sqlx::PgConnection,
	_config: &Settings,
) -> Result<(), Error> {
	//  "user" table
	query!(
		r#"
		ALTER TABLE "user"
		RENAME COLUMN backup_email_local
		TO recovery_email_local;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		RENAME COLUMN backup_email_domain_id
		TO recovery_email_domain_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		RENAME COLUMN backup_phone_country_code
		TO recovery_phone_country_code;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		RENAME COLUMN backup_phone_number
		TO recovery_phone_number;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		RENAME CONSTRAINT user_uq_backup_email_local_backup_email_domain_id
		TO user_uq_recovery_email_local_recovery_email_domain_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		RENAME CONSTRAINT user_uq_backup_phone_country_code_backup_phone_number
		TO user_uq_recovery_phone_country_code_recovery_phone_number;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		RENAME CONSTRAINT user_chk_bckp_eml_or_bckp_phn_present
		TO user_chk_rcvry_eml_or_rcvry_phn_present;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		RENAME CONSTRAINT user_chk_backup_email_is_lower_case
		TO user_chk_recovery_email_is_lower_case;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		RENAME CONSTRAINT user_chk_backup_phone_country_code_is_upper_case
		TO user_chk_recovery_phone_country_code_is_upper_case;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		RENAME CONSTRAINT user_fk_id_backup_email_local_backup_email_domain_id
		TO user_fk_id_recovery_email_local_recovery_email_domain_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		RENAME CONSTRAINT user_fk_id_backup_phone_country_code_backup_phone_number
		TO user_fk_id_recovery_phone_country_code_recovery_phone_number;
		"#
	)
	.execute(&mut *connection)
	.await?;

	// "user_to_sign_up" table

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME COLUMN backup_email_local
		TO recovery_email_local;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME COLUMN backup_email_domain_id
		TO recovery_email_domain_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME COLUMN backup_phone_country_code
		TO recovery_phone_country_code;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME COLUMN backup_phone_number
		TO recovery_phone_number;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME CONSTRAINT user_to_sign_up_chk_backup_email_is_lower_case
		TO user_to_sign_up_chk_recovery_email_is_lower_case;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME CONSTRAINT user_to_sign_up_fk_backup_email_domain_id
		TO user_to_sign_up_fk_recovery_email_domain_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME CONSTRAINT user_to_sign_up_fk_backup_phone_country_code
		TO user_to_sign_up_fk_recovery_phone_country_code;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME CONSTRAINT user_to_sign_up_chk_backup_phone_country_code_upper_case
		TO user_to_sign_up_chk_recovery_phone_country_code_upper_case;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE user_to_sign_up
		RENAME CONSTRAINT user_to_sign_up_chk_backup_details
		TO user_to_sign_up_chk_recovery_details;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
