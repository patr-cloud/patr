use api_models::utils::Uuid;
use k8s_openapi::{
	api::{
		apps::v1::{Deployment, DeploymentSpec},
		core::v1::{Container, PodSpec, PodTemplateSpec, ResourceRequirements},
	},
	apimachinery::pkg::api::resource::Quantity,
};
use kube::{
	api::PatchParams,
	config::{
		AuthInfo,
		Cluster,
		Context,
		Kubeconfig,
		NamedAuthInfo,
		NamedCluster,
		NamedContext,
	},
	error::ErrorResponse,
	Api,
	Config,
};
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
	refactor_resource_deletion(&mut *connection, config).await?;
	add_resource_requests_for_running_deployments(connection, config).await?;
	create_user_api_token_tables(connection, config).await?;
	update_payment_status_enum(connection, config).await?;

	Ok(())
}

async fn refactor_resource_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	remove_resource_name_column(connection, config).await?;

	refactor_static_site_deletion(connection, config).await?;
	refactor_secret_deletion(connection, config).await?;
	refactor_docker_repository_deletion(connection, config).await?;
	refactor_database_deletion(connection, config).await?;
	refactor_deployment_deletion(connection, config).await?;
	refactor_workspace_deletion(connection, config).await?;
	refactor_domain_deletion(connection, config).await?;
	refactor_managed_url_deletion(connection, config).await?;

	Ok(())
}

async fn remove_resource_name_column(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE resource
		DROP COLUMN name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn refactor_static_site_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE static_site
		ADD COLUMN deleted TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE static_site
		DROP CONSTRAINT static_site_uq_name_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			static_site
		SET
			deleted = COALESCE(
				(
					SELECT
						stop_time
					FROM
						static_sites_payment_history
					WHERE
						static_sites_payment_history.workspace_id = static_site.workspace_id
					ORDER BY
						stop_time DESC NULLS LAST
					LIMIT 1
				),
				NOW()
			)
		WHERE
			name LIKE CONCAT(
				'patr-deleted: ',
				'%-',
				REPLACE(id::TEXT, '-', '')
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	loop {
		let static_sites_with_patr_deleted = query!(
			r#"
			SELECT
				COUNT(*) as "count"
			FROM
				static_site
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					REPLACE(id::TEXT, '-', ''),
					'@%'
				);
			"#
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<i64, _>("count");

		if static_sites_with_patr_deleted <= 0 {
			break;
		}

		query!(
			r#"
			UPDATE
				static_site
			SET
				name = REPLACE(
					REPLACE(
						name,
						'patr-deleted: ',
						''
					),
					CONCAT(
						'-',
						REPLACE(id::TEXT, '-', '')
					),
					''
				)
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					'%-',
					REPLACE(id::TEXT, '-', '')
				);
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		CREATE UNIQUE INDEX
			static_site_uq_name_workspace_id
		ON
			static_site(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn refactor_secret_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE secret
		ADD COLUMN deleted TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE secret
		DROP CONSTRAINT secret_uq_workspace_id_name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			secret
		SET
			deleted = COALESCE(
				(
					SELECT
						stop_time
					FROM
						secrets_payment_history
					WHERE
						secrets_payment_history.workspace_id = secret.workspace_id
					ORDER BY
						stop_time DESC NULLS LAST
					LIMIT 1
				),
				NOW()
			)
		WHERE
			name LIKE CONCAT(
				'patr-deleted: ',
				REPLACE(id::TEXT, '-', ''),
				'@%'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	loop {
		let secrets_with_patr_deleted = query!(
			r#"
			SELECT
				COUNT(*) as "count"
			FROM
				secret
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					REPLACE(id::TEXT, '-', ''),
					'@%'
				);
			"#
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<i64, _>("count");

		if secrets_with_patr_deleted <= 0 {
			break;
		}

		query!(
			r#"
			UPDATE
				secret
			SET
				name = REPLACE(
					name,
					CONCAT(
						'patr-deleted: ',
						REPLACE(id::TEXT, '-', ''),
						'@'
					),
					''
				);
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		CREATE UNIQUE INDEX
			secret_uq_workspace_id_name
		ON
			secret(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn refactor_docker_repository_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE docker_registry_repository
		ADD COLUMN deleted TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE docker_registry_repository
		DROP CONSTRAINT docker_registry_repository_uq_workspace_id_name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			docker_registry_repository
		SET
			deleted = COALESCE(
				(
					SELECT
						stop_time
					FROM
						docker_repo_payment_history
					WHERE
						docker_repo_payment_history.workspace_id = docker_registry_repository.workspace_id
					ORDER BY
						stop_time DESC NULLS LAST
					LIMIT 1
				),
				NOW()
			)
		WHERE
			name LIKE CONCAT(
				'patr-deleted: ',
				'%-',
				REPLACE(id::TEXT, '-', '')
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	loop {
		let docker_repos_with_patr_deleted = query!(
			r#"
			SELECT
				COUNT(*) as "count"
			FROM
				docker_registry_repository
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					'%-',
					REPLACE(id::TEXT, '-', '')
				);
			"#
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<i64, _>("count");

		if docker_repos_with_patr_deleted <= 0 {
			break;
		}

		query!(
			r#"
			UPDATE
				docker_registry_repository
			SET
				name = REPLACE(
					REPLACE(
						name,
						'patr-deleted: ',
						''
					),
					CONCAT(
						'-',
						REPLACE(id::TEXT, '-', '')
					),
					''
				)
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					'%-',
					REPLACE(id::TEXT, '-', '')
				);
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		CREATE UNIQUE INDEX
			docker_registry_repository_uq_workspace_id_name
		ON
			docker_registry_repository(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn refactor_database_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE managed_database
		ADD COLUMN deleted TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database
		DROP CONSTRAINT managed_database_uq_name_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			managed_database
		SET
			deleted = COALESCE(
				(
					SELECT
						deletion_time
					FROM
						managed_database_payment_history
					WHERE
						database_id = managed_database.id
					LIMIT 1
				),
				NOW()
			)
		WHERE
			name LIKE CONCAT(
				'patr-deleted: ',
				'%-',
				id::TEXT
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	loop {
		let managed_databases_with_patr_deleted = query!(
			r#"
			SELECT
				COUNT(*) as "count"
			FROM
				managed_database
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					'%-',
					id::TEXT
				);
			"#
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<i64, _>("count");

		if managed_databases_with_patr_deleted <= 0 {
			break;
		}

		query!(
			r#"
			UPDATE
				managed_database
			SET
				name = REPLACE(
					REPLACE(
						name,
						'patr-deleted: ',
						''
					),
					CONCAT(
						'-',
						id::TEXT
					),
					''
				)
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					'%-',
					id::TEXT
				);
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		CREATE UNIQUE INDEX
			managed_database_uq_workspace_id_name
		ON
			managed_database(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn refactor_deployment_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE deployment
		ADD COLUMN deleted TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
		DROP CONSTRAINT deployment_uq_name_workspace_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			deployment
		SET
			deleted = COALESCE(
				(
					SELECT
						stop_time
					FROM
						deployment_payment_history
					WHERE
						deployment_id = deployment.id
					ORDER BY
						stop_time DESC
					LIMIT 1
				),
				NOW()
			)
		WHERE
			name LIKE CONCAT(
				'patr-deleted: ',
				REPLACE(id::TEXT, '-', ''),
				'@%'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	loop {
		let deployments_with_patr_deleted = query!(
			r#"
			SELECT
				COUNT(*) as "count"
			FROM
				deployment
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					REPLACE(id::TEXT, '-', ''),
					'@%'
				);
			"#
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<i64, _>("count");

		if deployments_with_patr_deleted <= 0 {
			break;
		}

		query!(
			r#"
			UPDATE
				deployment
			SET
				name = REPLACE(
					name,
					CONCAT(
						'patr-deleted: ',
						REPLACE(id::TEXT, '-', ''),
						'@'
					),
					''
				);
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		CREATE UNIQUE INDEX
			deployment_uq_workspace_id_name
		ON
			deployment(workspace_id, name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn refactor_workspace_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace
		ADD COLUMN deleted TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		DROP CONSTRAINT workspace_uq_name;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			workspace
		SET
			deleted = NOW()
		WHERE
			name LIKE CONCAT(
				'patr-deleted: ',
				REPLACE(id::TEXT, '-', ''),
				'@%'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	loop {
		let workspaces_with_patr_deleted = query!(
			r#"
			SELECT
				COUNT(*) as "count"
			FROM
				workspace
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					REPLACE(id::TEXT, '-', ''),
					'@%'
				);
			"#
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<i64, _>("count");

		if workspaces_with_patr_deleted <= 0 {
			break;
		}

		query!(
			r#"
			UPDATE
				workspace
			SET
				name = REPLACE(
					name,
					CONCAT(
						'patr-deleted: ',
						REPLACE(id::TEXT, '-', ''),
						'@'
					),
					''
				);
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		CREATE UNIQUE INDEX
			workspace_uq_name
		ON
			workspace(name)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn refactor_domain_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE domain
		ADD COLUMN deleted TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE domain
		DROP CONSTRAINT domain_uq_name_tld;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE domain
		DROP CONSTRAINT domain_chk_name_is_valid;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			domain
		SET
			deleted = COALESCE(
				(
					SELECT
						stop_time
					FROM
						domain_payment_history
					INNER JOIN
						resource
					ON
						domain.id = resource.id
					WHERE
						domain_payment_history.workspace_id = resource.owner_id
					ORDER BY
						stop_time DESC NULLS LAST
					LIMIT 1
				),
				NOW()
			)
		WHERE
			name LIKE CONCAT(
				'patr-deleted: ',
				REPLACE(id::TEXT, '-', ''),
				'@%'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	loop {
		let domains_with_patr_deleted = query!(
			r#"
			SELECT
				COUNT(*) as "count"
			FROM
				domain
			WHERE
				name LIKE CONCAT(
					'patr-deleted: ',
					REPLACE(id::TEXT, '-', ''),
					'@%'
				);
			"#
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<i64, _>("count");

		if domains_with_patr_deleted <= 0 {
			break;
		}

		query!(
			r#"
			UPDATE
				domain
			SET
				name = REPLACE(
					name,
					CONCAT(
						'patr-deleted: ',
						REPLACE(id::TEXT, '-', ''),
						'@'
					),
					''
				);
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		UPDATE
			domain
		SET
			name = REPLACE(name, CONCAT('.', tld), '');
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE domain
		ADD CONSTRAINT domain_chk_name_is_valid CHECK(
			name ~ '^(([a-z0-9])|([a-z0-9][a-z0-9-]*[a-z0-9]))$'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			domain_uq_name_tld
		ON
			domain(name, tld)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE domain
		ADD CONSTRAINT domain_uq_id_type_deleted UNIQUE(id, type, deleted);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// TODO: migration for personal_domain and workspace_domain too

	query!(
		r#"
		ALTER TABLE personal_domain
			ADD COLUMN deleted TIMESTAMPTZ
				CONSTRAINT personal_domain_chk_deletion CHECK(
					deleted IS NULL
				),
			DROP CONSTRAINT personal_domain_fk_id_domain_type,
			ADD CONSTRAINT personal_domain_fk_id_domain_type_deleted
				FOREIGN KEY(id, domain_type, deleted) REFERENCES domain(id, type, deleted);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn refactor_managed_url_deletion(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE managed_url
		ADD COLUMN deleted TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_url
		DROP CONSTRAINT managed_url_uq_sub_domain_domain_id_path;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_url
		DROP CONSTRAINT managed_url_chk_sub_domain_valid;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			managed_url
		SET
			deleted = COALESCE(
				(
					SELECT
						stop_time
					FROM
						managed_url_payment_history
					WHERE
						managed_url_payment_history.workspace_id = managed_url.workspace_id
					ORDER BY
						stop_time DESC NULLS LAST
					LIMIT 1
				),
				NOW()
			)
		WHERE
			sub_domain LIKE CONCAT(
				'patr-deleted: ',
				REPLACE(id::TEXT, '-', ''),
				'@%'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	loop {
		let managed_urls_with_patr_deleted = query!(
			r#"
			SELECT
				COUNT(*) as "count"
			FROM
				managed_url
			WHERE
				sub_domain LIKE CONCAT(
					'patr-deleted: ',
					REPLACE(id::TEXT, '-', ''),
					'@%'
				);
			"#
		)
		.fetch_one(&mut *connection)
		.await?
		.get::<i64, _>("count");

		if managed_urls_with_patr_deleted <= 0 {
			break;
		}

		query!(
			r#"
			UPDATE
				managed_url
			SET
				sub_domain = REPLACE(
					sub_domain,
					CONCAT(
						'patr-deleted: ',
						REPLACE(id::TEXT, '-', ''),
						'@'
					),
					''
				);
			"#
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		ALTER TABLE managed_url
		ADD CONSTRAINT managed_url_chk_sub_domain_valid
			CHECK(
				sub_domain ~ '^(([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])\.)*([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])$' OR
				sub_domain = '@'
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			managed_url_uq_sub_domain_domain_id_path
		ON
			managed_url(sub_domain, domain_id, path)
		WHERE
			deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_resource_requests_for_running_deployments(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	let running_deployments = query!(
		r#"
		SELECT
			workspace_id,
			id as "deployment_id"
		FROM
			deployment
		WHERE
			status = 'running' AND
			deleted IS NULL;
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| {
		(
			row.get::<Uuid, _>("workspace_id"),
			row.get::<Uuid, _>("deployment_id"),
		)
	})
	.collect::<Vec<_>>();

	if running_deployments.is_empty() {
		// added to skip CI error
		return Ok(());
	}

	// Kubernetes config
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

	let kubernetes_client = kube::Client::try_from(kubernetes_config)?;
	for (workspace_id, deployment_id) in running_deployments {
		let namespace = workspace_id.as_str();
		let deployment_name = format!("deployment-{}", deployment_id.as_str());

		let request_patch = Deployment {
			spec: Some(DeploymentSpec {
				template: PodTemplateSpec {
					spec: Some(PodSpec {
						containers: vec![Container {
							name: deployment_name.clone(),
							resources: Some(ResourceRequirements {
								requests: Some(
									[
										(
											"memory".to_string(),
											Quantity("25M".to_owned()),
										),
										(
											"cpu".to_string(),
											Quantity("50m".to_owned()),
										),
									]
									.into_iter()
									.collect(),
								),
								..Default::default()
							}),
							..Default::default()
						}],
						..Default::default()
					}),
					..Default::default()
				},
				..Default::default()
			}),
			..Default::default()
		};

		let result =
			Api::<Deployment>::namespaced(kubernetes_client.clone(), namespace)
				.patch(
					&deployment_name,
					&PatchParams::default(),
					&kube::api::Patch::Strategic(request_patch),
				)
				.await;

		match result {
			Ok(_deployment) => log::info!(
				"Successfully added k8s resource requests for deployment `{deployment_name}` in namespace `{namespace}`"
			),
			Err(kube::Error::Api(ErrorResponse { code: 404, .. })) => log::error!(
				"Deployment `{deployment_name}` not found in namespace `{namespace}`, hence skipped setting resource requests to it"
			),
			Err(err) => return Err(Error::new(Box::new(err))),
		}
	}

	Ok(())
}

async fn create_user_api_token_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE user_login
		RENAME TO web_login;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX user_login_idx_user_id
		RENAME TO web_login_idx_user_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			web_login_idx_login_id
		ON
			web_login(login_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_audit_log
		DROP CONSTRAINT workspace_audit_log_fk_login_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		DROP CONSTRAINT user_login_pk;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		DROP CONSTRAINT user_login_uq_login_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		DROP CONSTRAINT user_login_fk_user_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE USER_LOGIN_TYPE AS ENUM(
			'api_token',
			'web_login'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
			ADD COLUMN new_refresh_token TEXT,
			ADD COLUMN new_token_expiry TIMESTAMPTZ,
			ADD COLUMN new_created TIMESTAMPTZ,
			ADD COLUMN new_created_ip INET,
			ADD COLUMN new_created_location GEOMETRY,
			ADD COLUMN new_created_country TEXT,
			ADD COLUMN new_created_region TEXT,
			ADD COLUMN new_created_city TEXT,
			ADD COLUMN new_created_timezone TEXT,
			ADD COLUMN new_last_login TIMESTAMPTZ,
			ADD COLUMN new_last_activity TIMESTAMPTZ,
			ADD COLUMN new_last_activity_ip INET,
			ADD COLUMN new_last_activity_location GEOMETRY,
			ADD COLUMN new_last_activity_country TEXT,
			ADD COLUMN new_last_activity_region TEXT,
			ADD COLUMN new_last_activity_city TEXT,
			ADD COLUMN new_last_activity_timezone TEXT,
			ADD COLUMN new_last_activity_user_agent TEXT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			web_login
		SET
			new_refresh_token = refresh_token,
			new_token_expiry = token_expiry,
			new_created = created,
			new_created_ip = created_ip,
			new_created_location = created_location,
			new_created_country = created_country,
			new_created_region = created_region,
			new_created_city = created_city,
			new_created_timezone = created_timezone,
			new_last_login = last_login,
			new_last_activity = last_activity,
			new_last_activity_ip = last_activity_ip,
			new_last_activity_location = last_activity_location,
			new_last_activity_country = last_activity_country,
			new_last_activity_region = last_activity_region,
			new_last_activity_city = last_activity_city,
			new_last_activity_timezone = last_activity_timezone,
			new_last_activity_user_agent = last_activity_user_agent;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
			ALTER COLUMN new_refresh_token SET NOT NULL,
			DROP COLUMN refresh_token,
			ALTER COLUMN new_token_expiry SET NOT NULL,
			DROP COLUMN token_expiry,
			ALTER COLUMN new_created SET NOT NULL,
			DROP COLUMN created,
			ALTER COLUMN new_created_ip SET NOT NULL,
			DROP COLUMN created_ip,
			ALTER COLUMN new_created_location SET NOT NULL,
			DROP COLUMN created_location,
			ALTER COLUMN new_created_country SET NOT NULL,
			DROP COLUMN created_country,
			ALTER COLUMN new_created_region SET NOT NULL,
			DROP COLUMN created_region,
			ALTER COLUMN new_created_city SET NOT NULL,
			DROP COLUMN created_city,
			ALTER COLUMN new_created_timezone SET NOT NULL,
			DROP COLUMN created_timezone,
			ALTER COLUMN new_last_login SET NOT NULL,
			DROP COLUMN last_login,
			ALTER COLUMN new_last_activity SET NOT NULL,
			DROP COLUMN last_activity,
			ALTER COLUMN new_last_activity_ip SET NOT NULL,
			DROP COLUMN last_activity_ip,
			ALTER COLUMN new_last_activity_location SET NOT NULL,
			DROP COLUMN last_activity_location,
			ALTER COLUMN new_last_activity_country SET NOT NULL,
			DROP COLUMN last_activity_country,
			ALTER COLUMN new_last_activity_region SET NOT NULL,
			DROP COLUMN last_activity_region,
			ALTER COLUMN new_last_activity_city SET NOT NULL,
			DROP COLUMN last_activity_city,
			ALTER COLUMN new_last_activity_timezone SET NOT NULL,
			DROP COLUMN last_activity_timezone,
			ALTER COLUMN new_last_activity_user_agent SET NOT NULL,
			DROP COLUMN last_activity_user_agent;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_refresh_token TO refresh_token;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_token_expiry TO token_expiry;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_created TO created;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_created_ip TO created_ip;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_created_location TO created_location;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_created_country TO created_country;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_created_region TO created_region;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_created_city TO created_city;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_created_timezone TO created_timezone;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_last_login TO last_login;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_last_activity TO last_activity;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_last_activity_ip TO last_activity_ip;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_last_activity_location TO last_activity_location;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_last_activity_country TO last_activity_country;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_last_activity_region TO last_activity_region;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_last_activity_city TO last_activity_city;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_last_activity_timezone TO last_activity_timezone;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		RENAME COLUMN new_last_activity_user_agent TO last_activity_user_agent;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		ADD COLUMN login_type USER_LOGIN_TYPE NOT NULL
		GENERATED ALWAYS AS ('web_login') STORED;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_login(
			login_id UUID CONSTRAINT user_login_pk PRIMARY KEY,
			user_id UUID NOT NULL
				CONSTRAINT user_login_fk_user_id REFERENCES "user"(id),
			login_type USER_LOGIN_TYPE NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			CONSTRAINT user_login_uq_login_id_user_id UNIQUE(login_id, user_id),
			CONSTRAINT user_login_uq_login_id_user_id_login_type UNIQUE(
				login_id, user_id, login_type
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		INSERT INTO
			user_login(login_id, user_id, login_type, created)
		SELECT
			login_id, user_id, 'web_login', created
		FROM
			web_login;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_audit_log
		ADD CONSTRAINT workspace_audit_log_fk_login_id FOREIGN KEY(
			user_id, login_id
		)
		REFERENCES user_login(user_id, login_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE web_login
		ADD CONSTRAINT web_login_fk FOREIGN KEY(login_id, user_id, login_type)
		REFERENCES user_login(login_id, user_id, login_type);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token(
			token_id UUID CONSTRAINT user_api_token_pk PRIMARY KEY,
			name TEXT NOT NULL,
			user_id UUID NOT NULL,
			token_hash TEXT NOT NULL,
			token_nbf TIMESTAMPTZ, /* The token is not valid before this date */
			token_exp TIMESTAMPTZ, /* The token is not valid after this date */
			allowed_ips INET[],
			created TIMESTAMPTZ NOT NULL,
			revoked TIMESTAMPTZ,
			login_type USER_LOGIN_TYPE GENERATED ALWAYS AS ('api_token') STORED,
			CONSTRAINT user_api_token_token_id_user_id_uk UNIQUE(
				token_id, user_id
			),
			CONSTRAINT user_api_token_token_id_user_id_login_type_fk
				FOREIGN KEY(token_id, user_id, login_type)
					REFERENCES user_login(login_id, user_id, login_type)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			user_api_token_uq_name_user_id
		ON
			user_api_token(name, user_id)
		WHERE
			revoked IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		ADD CONSTRAINT workspace_uq_id_super_admin_id
		UNIQUE(id, super_admin_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_workspace_super_admin(
			token_id UUID NOT NULL,
			user_id UUID NOT NULL,
			workspace_id UUID NOT NULL,
			CONSTRAINT user_api_token_workspace_super_admin_fk_token
				FOREIGN KEY(token_id, user_id)
					REFERENCES user_api_token(token_id, user_id),
			CONSTRAINT user_api_token_workspace_super_admin_fk_workspace
				FOREIGN KEY(workspace_id, user_id)
					REFERENCES workspace(id, super_admin_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_resource_permission(
			token_id UUID NOT NULL
				CONSTRAINT user_api_token_resource_permission_fk_token_id
					REFERENCES user_api_token(token_id),
			workspace_id UUID NOT NULL,
			resource_id UUID NOT NULL,
			permission_id UUID NOT NULL
				CONSTRAINT user_api_token_resource_permission_fk_permission_id
					REFERENCES permission(id),
			CONSTRAINT user_api_token_resource_permission_workspace_id_resource_id
				FOREIGN KEY (workspace_id, resource_id)
					REFERENCES resource(owner_id, id),
			CONSTRAINT user_api_token_resource_permission_pk 
				PRIMARY KEY(token_id, permission_id, resource_id, workspace_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_api_token_resource_type_permission(
			token_id UUID NOT NULL
				CONSTRAINT user_api_token_resource_type_permission_fk_token_id
					REFERENCES user_api_token(token_id),
			workspace_id UUID NOT NULL
				CONSTRAINT user_api_token_resource_type_permission_fk_workspace_id
					REFERENCES workspace(id),
			resource_type_id UUID NOT NULL
				CONSTRAINT user_api_token_resource_type_permission_fk_resource_type_id
					REFERENCES resource_type(id),
			permission_id UUID NOT NULL
				CONSTRAINT user_api_token_resource_type_permission_fk_permission_id
					REFERENCES permission(id),
			CONSTRAINT user_api_token_resource_type_permission_pk
				PRIMARY KEY(token_id, permission_id, resource_type_id, workspace_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn update_payment_status_enum(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TYPE PAYMENT_STATUS
		ADD VALUE 'pending';
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
