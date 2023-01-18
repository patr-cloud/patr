use std::net::IpAddr;

use api_models::utils::Uuid;
use chrono::{DateTime, Utc};
use k8s_openapi::api::{
	apps::v1::Deployment,
	autoscaling::v1::HorizontalPodAutoscaler,
	core::v1::{ConfigMap, Service},
	networking::v1::Ingress,
};
use kube::{
	api::DeleteParams,
	config::{
		AuthInfo,
		Cluster,
		Context,
		Kubeconfig,
		NamedAuthInfo,
		NamedCluster,
		NamedContext,
	},
	Api,
	Config,
};
use sqlx::Row;

use crate::{
	migrate_query as query,
	service::ext_traits::DeleteOpt,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	delete_deployment_with_invalid_image_name(connection, config).await?;
	validate_image_name_for_deployment(connection, config).await?;
	permission_change_for_rbac_v1(connection, config).await?;
	add_permission_for_write_ci_file_to_repo(connection, config).await?;
	reset_permission_order(connection, config).await?;
	Ok(())
}

pub(super) async fn delete_deployment_with_invalid_image_name(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	let deployments = query!(
		r#"
		SELECT
			*
		FROM
			deployment
		WHERE
			image_name IS NOT NULL AND
			image_name !~ '^(([a-z0-9]+)(((?:[._]|__|[-]*)([a-z0-9]+))*)?)(((\/)(([a-z0-9]+)(((?:[._]|__|[-]*)([a-z0-9]+))*)?))*)?$';
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (
		row.get::<Uuid, _>("id"),
		row.get::<Uuid, _>("workspace_id"),
		row.get::<Uuid, _>("region"),
		row.get::<Option<DateTime<Utc>>, _>("deleted"),
	))
	.collect::<Vec<_>>();

	for (deployment_id, workspace_id, region_id, deleted) in deployments {
		query!(
			r#"
			UPDATE
				deployment
			SET
				image_name = 'undefined'
			WHERE
				id = $1;
			"#,
			&deployment_id
		)
		.execute(&mut *connection)
		.await?;
		if deleted.is_some() {
			// No need to delete the deployment
			continue;
		}
		delete_deployment(
			connection,
			&deployment_id,
			&workspace_id,
			&region_id,
			config,
		)
		.await?
	}

	Ok(())
}

pub(super) async fn validate_image_name_for_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE deployment
		ADD CONSTRAINT deployment_chk_image_name_is_valid
		CHECK (
			image_name ~ '^(([a-z0-9]+)(((?:[._]|__|[-]*)([a-z0-9]+))*)?)(((\/)(([a-z0-9]+)(((?:[._]|__|[-]*)([a-z0-9]+))*)?))*)?$'
		);
	"#
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}

async fn delete_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	deployment_workspace_id: &Uuid,
	region_id: &Uuid,
	config: &Settings,
) -> Result<(), Error> {
	let (
		ready,
		workspace_id,
		kubernetes_cluster_url,
		kubernetes_auth_username,
		kubernetes_auth_token,
		kubernetes_ca_data,
		kubernetes_ingress_ip_addr,
	) = query!(
		r#"
		SELECT
			*
		FROM
			deployment_region
		WHERE
			id = $1;
		"#,
		region_id,
		deployment_workspace_id,
	)
	.fetch_one(&mut *connection)
	.await
	.map(|row| {
		(
			row.get::<bool, _>("ready"),
			row.get::<Option<Uuid>, _>("workspace_id"),
			row.get::<Option<String>, _>("kubernetes_cluster_url"),
			row.get::<Option<String>, _>("kubernetes_auth_username"),
			row.get::<Option<String>, _>("kubernetes_auth_token"),
			row.get::<Option<String>, _>("kubernetes_ca_data"),
			row.get::<Option<IpAddr>, _>("kubernetes_ingress_ip_addr"),
		)
	})?;

	if workspace_id.is_none() {
		query!(
			r#"
			UPDATE
				deployment_payment_history
			SET
				stop_time = NOW()
			WHERE
				deployment_id = $1 AND
				stop_time IS NULL;
			"#,
			deployment_id
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		UPDATE
			deployment
		SET
			status = 'deleted',
			deleted = NOW()
		WHERE
			deployment_id = $1;
		"#,
		deployment_id
	)
	.execute(&mut *connection)
	.await?;

	let kube_config = if workspace_id.is_none() {
		get_kube_config(
			&config.kubernetes.cluster_url,
			&config.kubernetes.certificate_authority_data,
			&config.kubernetes.auth_username,
			&config.kubernetes.auth_token,
		)
		.await?
	} else {
		match (
			ready,
			kubernetes_cluster_url,
			kubernetes_auth_username,
			kubernetes_auth_token,
			kubernetes_ca_data,
			kubernetes_ingress_ip_addr,
		) {
			(
				true,
				Some(cluster_url),
				Some(auth_username),
				Some(auth_token),
				Some(certificate_authority_data),
				Some(_),
			) => {
				get_kube_config(
					&cluster_url,
					&certificate_authority_data,
					&auth_username,
					&auth_token,
				)
				.await?
			}
			_ => {
				log::info!("cluster {region_id} is not yet initialized");
				return Ok(());
			}
		}
	};
	let kubernetes_client = kube::Client::try_from(kube_config)?;

	Api::<Deployment>::namespaced(
		kubernetes_client.clone(),
		deployment_workspace_id.as_str(),
	)
	.delete_opt(
		&format!("deployment-{}", deployment_id),
		&DeleteParams::default(),
	)
	.await?;

	Api::<ConfigMap>::namespaced(
		kubernetes_client.clone(),
		deployment_workspace_id.as_str(),
	)
	.delete_opt(
		&format!("config-mount-{}", deployment_id),
		&DeleteParams::default(),
	)
	.await?;

	Api::<Service>::namespaced(
		kubernetes_client.clone(),
		deployment_workspace_id.as_str(),
	)
	.delete_opt(
		&format!("service-{}", deployment_id),
		&DeleteParams::default(),
	)
	.await?;

	Api::<HorizontalPodAutoscaler>::namespaced(
		kubernetes_client.clone(),
		deployment_workspace_id.as_str(),
	)
	.delete_opt(&format!("hpa-{}", deployment_id), &DeleteParams::default())
	.await?;

	Api::<Ingress>::namespaced(
		kubernetes_client,
		deployment_workspace_id.as_str(),
	)
	.delete_opt(
		&format!("ingress-{}", deployment_id),
		&DeleteParams::default(),
	)
	.await?;

	Ok(())
}

async fn get_kube_config(
	cluster_url: &str,
	certificate_authority_data: &str,
	auth_username: &str,
	auth_token: &str,
) -> Result<Config, Error> {
	let kube_config = Config::from_custom_kubeconfig(
		Kubeconfig {
			api_version: Some("v1".to_string()),
			kind: Some("Config".to_string()),
			clusters: vec![NamedCluster {
				name: "kubernetesCluster".to_owned(),
				cluster: Cluster {
					server: cluster_url.to_string(),
					certificate_authority_data: Some(
						certificate_authority_data.to_string(),
					),
					insecure_skip_tls_verify: None,
					certificate_authority: None,
					proxy_url: None,
					extensions: None,
				},
			}],
			auth_infos: vec![NamedAuthInfo {
				name: auth_username.to_string(),
				auth_info: AuthInfo {
					token: Some(auth_token.to_string().into()),
					..Default::default()
				},
			}],
			contexts: vec![NamedContext {
				name: "kubernetesContext".to_owned(),
				context: Context {
					cluster: "kubernetesCluster".to_owned(),
					user: auth_username.to_string(),
					extensions: None,
					namespace: None,
				},
			}],
			current_context: Some("kubernetesContext".to_owned()),
			preferences: None,
			extensions: None,
		},
		&Default::default(),
	)
	.await?;

	Ok(kube_config)
}

pub async fn permission_change_for_rbac_v1(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// add permissions for CI
	for &permission in [
		"workspace::ci::git_provider::repo::info",
		"workspace::ci::git_provider::repo::build::start",
		"workspace::ci::git_provider::repo::build::list",
		"workspace::ci::git_provider::repo::build::cancel",
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

	query!(
		r#"
		UPDATE
			permission
		SET
			name = 'workspace::ci::git_provider::repo::build::info'
		WHERE
			name = 'workspace::ci::git_provider::repo::build::view';
		"#,
	)
	.execute(&mut *connection)
	.await?;

	// add billing permissions
	for &permission in [
		"workspace::billing::info",
		"workspace::billing::make_payment",
		"workspace::billing::payment_method::add",
		"workspace::billing::payment_method::delete",
		"workspace::billing::payment_method::list",
		"workspace::billing::payment_method::edit",
		"workspace::billing::billing_address::add",
		"workspace::billing::billing_address::delete",
		"workspace::billing::billing_address::info",
		"workspace::billing::billing_address::edit",
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

// todo: move migrations to 13.2
pub async fn add_permission_for_write_ci_file_to_repo(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// add permissions for CI
	let permission = "workspace::ci::git_provider::repo::write";

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

	Ok(())
}

async fn reset_permission_order(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	for permission in [
		// domain
		"workspace::domain::list",
		"workspace::domain::add",
		"workspace::domain::viewDetails",
		"workspace::domain::verify",
		"workspace::domain::delete",
		// dns
		"workspace::domain::dnsRecord::list",
		"workspace::domain::dnsRecord::add",
		"workspace::domain::dnsRecord::edit",
		"workspace::domain::dnsRecord::delete",
		// deployment
		"workspace::infrastructure::deployment::list",
		"workspace::infrastructure::deployment::create",
		"workspace::infrastructure::deployment::info",
		"workspace::infrastructure::deployment::delete",
		"workspace::infrastructure::deployment::edit",
		// upgrade path
		"workspace::infrastructure::upgradePath::list",
		"workspace::infrastructure::upgradePath::create",
		"workspace::infrastructure::upgradePath::info",
		"workspace::infrastructure::upgradePath::delete",
		"workspace::infrastructure::upgradePath::edit",
		// managed url
		"workspace::infrastructure::managedUrl::list",
		"workspace::infrastructure::managedUrl::create",
		"workspace::infrastructure::managedUrl::edit",
		"workspace::infrastructure::managedUrl::delete",
		// managed database
		"workspace::infrastructure::managedDatabase::create",
		"workspace::infrastructure::managedDatabase::list",
		"workspace::infrastructure::managedDatabase::delete",
		"workspace::infrastructure::managedDatabase::info",
		// static site
		"workspace::infrastructure::staticSite::list",
		"workspace::infrastructure::staticSite::create",
		"workspace::infrastructure::staticSite::info",
		"workspace::infrastructure::staticSite::delete",
		"workspace::infrastructure::staticSite::edit",
		// docker registry
		"workspace::dockerRegistry::create",
		"workspace::dockerRegistry::list",
		"workspace::dockerRegistry::delete",
		"workspace::dockerRegistry::info",
		"workspace::dockerRegistry::push",
		"workspace::dockerRegistry::pull",
		// secret
		"workspace::secret::list",
		"workspace::secret::create",
		"workspace::secret::edit",
		"workspace::secret::delete",
		// role
		"workspace::rbac::role::list",
		"workspace::rbac::role::create",
		"workspace::rbac::role::edit",
		"workspace::rbac::role::delete",
		// user
		"workspace::rbac::user::list",
		"workspace::rbac::user::add",
		"workspace::rbac::user::remove",
		"workspace::rbac::user::updateRoles",
		// region
		"workspace::region::list",
		"workspace::region::add",
		// ci
		"workspace::ci::git_provider::connect",
		"workspace::ci::git_provider::disconnect",
		"workspace::ci::git_provider::repo::activate",
		"workspace::ci::git_provider::repo::deactivate",
		"workspace::ci::git_provider::repo::list",
		"workspace::ci::git_provider::repo::info",
		"workspace::ci::git_provider::repo::write",
		"workspace::ci::git_provider::repo::build::list",
		"workspace::ci::git_provider::repo::build::cancel",
		"workspace::ci::git_provider::repo::build::info",
		"workspace::ci::git_provider::repo::build::start",
		"workspace::ci::git_provider::repo::build::restart",
		// billling
		"workspace::billing::info",
		"workspace::billing::make_payment",
		"workspace::billing::payment_method::add",
		"workspace::billing::payment_method::delete",
		"workspace::billing::payment_method::list",
		"workspace::billing::payment_method::edit",
		"workspace::billing::billing_address::add",
		"workspace::billing::billing_address::delete",
		"workspace::billing::billing_address::info",
		"workspace::billing::billing_address::edit",
		// workspace
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
