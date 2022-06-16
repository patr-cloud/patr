use api_models::utils::Uuid;
use k8s_openapi::api::autoscaling::v1::{
	CrossVersionObjectReference,
	HorizontalPodAutoscaler,
	HorizontalPodAutoscalerSpec,
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

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	add_github_permissions(&mut *connection, config).await?;
	add_alert_emails(&mut *connection, config).await?;
	update_workspace_with_ci_columns(&mut *connection, config).await?;
	reset_permission_order(&mut *connection, config).await?;
	add_hpa_to_existing_deployments(&mut *connection, config).await?;
	update_deployment_with_probe_column(&mut *connection, config).await?;
	add_workspace_credit_column(&mut *connection, config).await?;
	rbac_related_migrations(&mut *connection, config).await?;

	Ok(())
}

async fn add_github_permissions(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	for &permission in [
		"workspace::ci::github::connect",
		"workspace::ci::github::activate",
		"workspace::ci::github::deactivate",
		"workspace::ci::github::viewBuilds",
		"workspace::ci::github::restartBuilds",
		"workspace::ci::github::disconnect",
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
		.fetch_optional(&mut *connection)
		.await?;
	}

	Ok(())
}

async fn update_workspace_with_ci_columns(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace
			ADD COLUMN drone_username TEXT
				CONSTRAINT workspace_uq_drone_username UNIQUE,
			ADD COLUMN drone_token TEXT
				CONSTRAINT workspace_chk_drone_token_is_not_null
					CHECK(
						(
							drone_username IS NULL AND
							drone_token IS NULL
						) OR (
							drone_username IS NOT NULL AND
							drone_token IS NOT NULL
						)
					);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn reset_permission_order(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	for permission in [
		// Domain permissions
		"workspace::domain::list",
		"workspace::domain::add",
		"workspace::domain::viewDetails",
		"workspace::domain::verify",
		"workspace::domain::delete",
		// Dns record permissions
		"workspace::domain::dnsRecord::list",
		"workspace::domain::dnsRecord::add",
		"workspace::domain::dnsRecord::edit",
		"workspace::domain::dnsRecord::delete",
		// Deployment permissions
		"workspace::infrastructure::deployment::list",
		"workspace::infrastructure::deployment::create",
		"workspace::infrastructure::deployment::info",
		"workspace::infrastructure::deployment::delete",
		"workspace::infrastructure::deployment::edit",
		// Upgrade path permissions
		"workspace::infrastructure::upgradePath::list",
		"workspace::infrastructure::upgradePath::create",
		"workspace::infrastructure::upgradePath::info",
		"workspace::infrastructure::upgradePath::delete",
		"workspace::infrastructure::upgradePath::edit",
		// Managed URL permissions
		"workspace::infrastructure::managedUrl::list",
		"workspace::infrastructure::managedUrl::create",
		"workspace::infrastructure::managedUrl::edit",
		"workspace::infrastructure::managedUrl::delete",
		// Managed database permissions
		"workspace::infrastructure::managedDatabase::create",
		"workspace::infrastructure::managedDatabase::list",
		"workspace::infrastructure::managedDatabase::delete",
		"workspace::infrastructure::managedDatabase::info",
		// Static site permissions
		"workspace::infrastructure::staticSite::list",
		"workspace::infrastructure::staticSite::create",
		"workspace::infrastructure::staticSite::info",
		"workspace::infrastructure::staticSite::delete",
		"workspace::infrastructure::staticSite::edit",
		// Docker registry permissions
		"workspace::dockerRegistry::create",
		"workspace::dockerRegistry::list",
		"workspace::dockerRegistry::delete",
		"workspace::dockerRegistry::info",
		"workspace::dockerRegistry::push",
		"workspace::dockerRegistry::pull",
		// Secret permissions
		"workspace::secret::list",
		"workspace::secret::create",
		"workspace::secret::edit",
		"workspace::secret::delete",
		// RBAC Role permissions
		"workspace::rbac::role::list",
		"workspace::rbac::role::create",
		"workspace::rbac::role::edit",
		"workspace::rbac::role::delete",
		// RBAC User permissions
		"workspace::rbac::user::list",
		"workspace::rbac::user::add",
		"workspace::rbac::user::remove",
		"workspace::rbac::user::updateRoles",
		// CI permissions
		"workspace::ci::github::connect",
		"workspace::ci::github::activate",
		"workspace::ci::github::deactivate",
		"workspace::ci::github::viewBuilds",
		"workspace::ci::github::restartBuilds",
		"workspace::ci::github::disconnect",
		// Workspace permissions
		"workspace::edit",
		"workspace::delete",
	] {
		query!(
			r#"
			UPDATE
				permission
			SET
				name = CONCAT('test::', name)
			WHERE
				name = $1;
			"#,
			permission,
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			UPDATE
				permission
			SET
				name = $1
			WHERE
				name = CONCAT('test::', $1);
			"#,
			&permission,
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

async fn add_alert_emails(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace
			ADD COLUMN alert_emails VARCHAR(320) [] NOT NULL 
			DEFAULT ARRAY[]::VARCHAR[];
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		ALTER COLUMN alert_emails DROP DEFAULT;
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			workspace w1
		SET
			alert_emails = (
				SELECT 
					ARRAY_AGG(CONCAT(
						"user".recovery_email_local,
						'@',
						domain.name,
						'.',
						domain.tld
					))
				FROM 
					workspace w2
				INNER JOIN
					"user"
				ON
					"user".id = w2.super_admin_id
				INNER JOIN
					domain
				ON
					"user".recovery_email_domain_id = domain.id
				WHERE
					w2.id = w1.id
			);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

// HPA - Horizontal Pod Autoscaler
async fn add_hpa_to_existing_deployments(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	let deployments = query!(
		r#"
		SELECT
			id,
			workspace_id,
			min_horizontal_scale,
			max_horizontal_scale
		FROM
			deployment
		WHERE	
			status != 'deleted' OR
			status != 'stopped';
		"#,
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| {
		(
			row.get::<Uuid, _>("id"),
			row.get::<Uuid, _>("workspace_id"),
			row.get::<i16, _>("min_horizontal_scale"),
			row.get::<i16, _>("max_horizontal_scale"),
		)
	})
	.collect::<Vec<_>>();

	if deployments.is_empty() {
		return Ok(());
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
	.await?;

	let kubernetes_client = kube::Client::try_from(kubernetes_config)?;

	for (id, workspace_id, min_horizontal_scale, max_horizontal_scale) in
		deployments
	{
		// HPA - horizontal pod autoscaler
		let kubernetes_hpa = HorizontalPodAutoscaler {
			metadata: ObjectMeta {
				name: Some(format!("hpa-{}", id)),
				namespace: Some(workspace_id.to_string()),
				..ObjectMeta::default()
			},
			spec: Some(HorizontalPodAutoscalerSpec {
				scale_target_ref: CrossVersionObjectReference {
					api_version: Some("apps/v1".to_string()),
					kind: "Deployment".to_string(),
					name: format!("deployment-{}", id),
				},
				min_replicas: Some(min_horizontal_scale.into()),
				max_replicas: max_horizontal_scale.into(),
				target_cpu_utilization_percentage: Some(80),
			}),
			..HorizontalPodAutoscaler::default()
		};

		let hpa_api = Api::<HorizontalPodAutoscaler>::namespaced(
			kubernetes_client.clone(),
			workspace_id.as_str(),
		);

		hpa_api
			.patch(
				&format!("hpa-{}", id),
				&PatchParams::apply(&format!("hpa-{}", id)),
				&Patch::Apply(kubernetes_hpa),
			)
			.await?;
	}

	Ok(())
}

async fn update_deployment_with_probe_column(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE deployment_exposed_port
			ADD CONSTRAINT deployment_exposed_port_uq_deployment_id_port_port_type
				UNIQUE(deployment_id, port, port_type),
			ALTER COLUMN deployment_id SET NOT NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment
			ADD COLUMN startup_probe_port INTEGER,
			ADD COLUMN startup_probe_path VARCHAR(255),
			ADD COLUMN startup_probe_port_type EXPOSED_PORT_TYPE,
			ADD COLUMN liveness_probe_port INTEGER,
			ADD COLUMN liveness_probe_path VARCHAR(255),
			ADD COLUMN liveness_probe_port_type EXPOSED_PORT_TYPE,
			ADD CONSTRAINT deployment_fk_deployment_id_startup_port_startup_port_type
				FOREIGN KEY (id, startup_probe_port, startup_probe_port_type)
					REFERENCES deployment_exposed_port(deployment_id, port, port_type)
					DEFERRABLE INITIALLY IMMEDIATE,
			ADD CONSTRAINT deployment_fk_deployment_id_liveness_port_liveness_port_type
				FOREIGN KEY (id, liveness_probe_port, liveness_probe_port_type)
					REFERENCES deployment_exposed_port(deployment_id, port, port_type)
					DEFERRABLE INITIALLY IMMEDIATE,
			ADD CONSTRAINT deployment_chk_startup_probe_is_valid CHECK(
					(
						startup_probe_port IS NULL AND
						startup_probe_path IS NULL AND
						startup_probe_port_type IS NULL
					) OR (
						startup_probe_port IS NOT NULL AND
						startup_probe_path IS NOT NULL AND
						startup_probe_port_type IS NOT NULL
					)
				),
			ADD CONSTRAINT deployment_chk_liveness_probe_is_valid CHECK(
					(
						liveness_probe_port IS NULL AND
						liveness_probe_path IS NULL AND
						liveness_probe_port_type IS NULL
					) OR (
						liveness_probe_port IS NOT NULL AND
						liveness_probe_path IS NOT NULL AND
						liveness_probe_port_type IS NOT NULL
					)
				),
			ADD CONSTRAINT deployment_chk_startup_probe_port_type_is_http
				CHECK(
					startup_probe_port_type = 'http'
				),
			ADD CONSTRAINT deployment_chk_liveness_probe_port_type_is_http
				CHECK(
					liveness_probe_port_type = 'http'
				);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn add_workspace_credit_column(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE workspace_credits(
			workspace_id UUID NOT NULL,
			credits BIGINT NOT NULL DEFAULT 0,
			metadata JSON NOT NULL,
			date TIMESTAMPTZ NOT NULL,
			CONSTRAINT workspace_credits_fk_workspace_id FOREIGN KEY (workspace_id) REFERENCES workspace(id)
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn rbac_related_migrations(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	rename_role_permissions_resource_tables(&mut *connection, config).await?;
	create_role_block_permissions_resource_table(&mut *connection, config)
		.await?;
	add_validation_for_permissions_on_resource(&mut *connection, config)
		.await?;

	Ok(())
}

async fn rename_role_permissions_resource_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE
			role_permissions_resource
		RENAME TO
			role_allow_permissions_resource;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_permissions_resource_type
		RENAME TO
			role_allow_permissions_resource_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource_type
		RENAME CONSTRAINT
			role_permissions_resource_type_fk_role_id
		TO
			role_allow_permissions_resource_type_fk_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource_type
		RENAME CONSTRAINT
			role_permissions_resource_type_fk_permission_id
		TO
			role_allow_permissions_resource_type_fk_permission_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource_type
		RENAME CONSTRAINT
			role_permissions_resource_type_fk_resource_type_id
		TO
			role_allow_permissions_resource_type_fk_resource_type_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource_type
		RENAME CONSTRAINT
			role_permissions_resource_type_pk
		TO
			role_allow_permissions_resource_type_pk;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource
		RENAME CONSTRAINT
			role_permissions_resource_fk_role_id
		TO
			role_allow_permissions_resource_fk_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource
		RENAME CONSTRAINT
			role_permissions_resource_fk_permission_id
		TO
			role_allow_permissions_resource_fk_permission_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource
		RENAME CONSTRAINT
			role_permissions_resource_fk_resource_id
		TO
			role_allow_permissions_resource_fk_resource_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE
			role_allow_permissions_resource
		RENAME CONSTRAINT
			role_permissions_resource_pk
		TO
			role_allow_permissions_resource_pk;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX
			role_permissions_resource_type_idx_role_id
		RENAME TO
			role_allow_permissions_resource_type_idx_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX
			role_permissions_resource_type_idx_role_id_resource_type_id
		RENAME TO
			role_allow_permissions_resource_type_idx_roleid_resourcetypeid;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX
			role_permissions_resource_idx_role_id
		RENAME TO
			role_allow_permissions_resource_idx_role_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER INDEX
			role_permissions_resource_idx_role_id_resource_id
		RENAME TO
			role_allow_permissions_resource_idx_role_id_resource_id;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn create_role_block_permissions_resource_table(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE role_block_permissions_resource(
			role_id UUID
				CONSTRAINT role_block_permissions_resource_fk_role_id
					REFERENCES role(id),
			permission_id UUID
				CONSTRAINT role_block_permissions_resource_fk_permission_id
					REFERENCES permission(id),
			resource_id UUID
				CONSTRAINT role_block_permissions_resource_fk_resource_id
					REFERENCES resource(id),
			CONSTRAINT role_block_permissions_resource_pk
				PRIMARY KEY(role_id, permission_id, resource_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_block_permissions_resource_idx_role_id
		ON
			role_block_permissions_resource(role_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			role_block_permissions_resource_idx_role_id_resource_id
		ON
			role_block_permissions_resource(role_id, resource_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_validation_for_permissions_on_resource(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// TODO: Make sure rbac is not used in db,
	// Else need to add validation for exisiting data

	// create permission to resource mapping function
	query!(
		r#"
		CREATE OR REPLACE FUNCTION validate_permission_to_resource_mapping(
			permission_name TEXT,
			resource_type_name TEXT
		) RETURNS BOOLEAN AS $$
		SELECT CASE
			resource_type_name
			WHEN 'workspace' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::domain::list',
							'workspace::domain::add',

							'workspace::infrastructure::deployment::list',
							'workspace::infrastructure::deployment::create',

							'workspace::infrastructure::managedUrl::list',
							'workspace::infrastructure::managedUrl::create',

							'workspace::infrastructure::managedDatabase::create',
							'workspace::infrastructure::managedDatabase::list',

							'workspace::dockerRegistry::create',
							'workspace::dockerRegistry::list',

							'workspace::secret::list',
							'workspace::secret::create',

							'workspace::infrastructure::staticSite::list',
							'workspace::infrastructure::staticSite::create',

							'workspace::infrastructure::upgradePath::list',
							'workspace::infrastructure::upgradePath::create',

							'workspace::rbac::role::list',
							'workspace::rbac::role::create',
							'workspace::rbac::role::edit',
							'workspace::rbac::role::delete',

							'workspace::rbac::user::list',
							'workspace::rbac::user::add',
							'workspace::rbac::user::remove',
							'workspace::rbac::user::updateRoles',

							'workspace::edit',
							'workspace::delete',

							'workspace::ci::github::connect',
							'workspace::ci::github::activate',
							'workspace::ci::github::deactivate',
							'workspace::ci::github::viewBuilds',
							'workspace::ci::github::restartBuilds',
							'workspace::ci::github::disconnect'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'domain' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::domain::viewDetails',
							'workspace::domain::verify',
							'workspace::domain::delete',

							'workspace::domain::dnsRecord::list',
							'workspace::domain::dnsRecord::add'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'dnsRecord' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::domain::dnsRecord::edit',
							'workspace::domain::dnsRecord::delete'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'dockerRepository' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::dockerRegistry::delete',
							'workspace::dockerRegistry::info',
							'workspace::dockerRegistry::push',
							'workspace::dockerRegistry::pull'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'managedDatabase' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::infrastructure::managedDatabase::delete',
							'workspace::infrastructure::managedDatabase::info'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'deployment' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::infrastructure::deployment::info',
							'workspace::infrastructure::deployment::delete',
							'workspace::infrastructure::deployment::edit'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'staticSite' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::infrastructure::staticSite::info',
							'workspace::infrastructure::staticSite::delete',
							'workspace::infrastructure::staticSite::edit'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'deploymentUpgradePath' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::infrastructure::upgradePath::info',
							'workspace::infrastructure::upgradePath::delete',
							'workspace::infrastructure::upgradePath::edit'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'managedUrl' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::infrastructure::managedUrl::edit',
							'workspace::infrastructure::managedUrl::delete'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			WHEN 'secret' THEN (
				CASE
					WHEN permission_name = ANY (
						ARRAY [
							'workspace::secret::edit',
							'workspace::secret::delete'
						]
					) THEN TRUE
					ELSE FALSE
				END
			)
			ELSE FALSE
		END;
		$$ LANGUAGE SQL IMMUTABLE STRICT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	// create triggers for role_allow_permissions_resource
	query!(
		r#"
		CREATE OR REPLACE FUNCTION
			role_allow_permissions_resource_check()
		RETURNS TRIGGER AS $$
		DECLARE
			permission_name TEXT;
			resource_type_name TEXT;
		BEGIN
			SELECT
				permission.name INTO permission_name
			FROM
				permission
			WHERE
				permission.id = NEW.permission_id;

			SELECT
				resource_type.name INTO resource_type_name
			FROM
				resource_type
			JOIN
				resource ON resource.resource_type_id = resource_type.id
			WHERE
				resource.id = NEW.resource_id;

			IF validate_permission_to_resource_mapping(permission_name, resource_type_name) THEN
				RETURN NEW;
			END IF;

			RAISE EXCEPTION 'Invalid permission is provided for resource';
		END
		$$ LANGUAGE PLPGSQL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TRIGGER IF EXISTS
			role_allow_permissions_resource_check_trigger
		ON
			role_allow_permissions_resource;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TRIGGER
			role_allow_permissions_resource_check_trigger
		BEFORE INSERT OR UPDATE ON
			role_allow_permissions_resource
		FOR EACH ROW EXECUTE FUNCTION
			role_allow_permissions_resource_check();
		"#
	)
	.execute(&mut *connection)
	.await?;

	// create triggers for role_block_permissions_resource
	query!(
		r#"
		CREATE OR REPLACE FUNCTION
			role_block_permissions_resource_check()
		RETURNS TRIGGER AS $$
		DECLARE
			permission_name TEXT;
			resource_type_name TEXT;
		BEGIN
			SELECT
				permission.name INTO permission_name
			FROM
				permission
			WHERE
				permission.id = NEW.permission_id;

			SELECT
				resource_type.name INTO resource_type_name
			FROM
				resource_type
			JOIN
				resource ON resource.resource_type_id = resource_type.id
			WHERE
				resource.id = NEW.resource_id;

			IF validate_permission_to_resource_mapping(permission_name, resource_type_name) THEN
				RETURN NEW;
			END IF;

			RAISE EXCEPTION 'Invalid permission is provided for resource';
		END
		$$ LANGUAGE PLPGSQL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TRIGGER IF EXISTS
			role_block_permissions_resource_check_trigger
		ON
			role_block_permissions_resource;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TRIGGER
			role_block_permissions_resource_check_trigger
		BEFORE INSERT OR UPDATE ON
			role_block_permissions_resource
		FOR EACH ROW EXECUTE FUNCTION
			role_block_permissions_resource_check();
		"#
	)
	.execute(&mut *connection)
	.await?;

	// create triggers for role_allow_permissions_resource_type
	query!(
		r#"
		CREATE OR REPLACE FUNCTION
			role_allow_permissions_resource_type_check()
		RETURNS TRIGGER AS $$
		DECLARE
			permission_name TEXT;
			resource_type_name TEXT;
		BEGIN
			SELECT
				permission.name INTO permission_name
			FROM
				permission
			WHERE
				permission.id = NEW.permission_id;

			SELECT
				resource_type.name INTO resource_type_name
			FROM
				resource_type
			WHERE
				resource_type.id = NEW.resource_type_id;

			IF validate_permission_to_resource_mapping(permission_name, resource_type_name) THEN
				RETURN NEW;
			END IF;

			RAISE EXCEPTION 'Invalid permission is provided for resource';
		END
		$$ LANGUAGE PLPGSQL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		DROP TRIGGER IF EXISTS
			role_allow_permissions_resource_type_check_trigger
		ON
			role_allow_permissions_resource_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TRIGGER
			role_allow_permissions_resource_type_check_trigger
		BEFORE INSERT OR UPDATE ON
			role_allow_permissions_resource_type
		FOR EACH ROW EXECUTE FUNCTION
			role_allow_permissions_resource_type_check();
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
