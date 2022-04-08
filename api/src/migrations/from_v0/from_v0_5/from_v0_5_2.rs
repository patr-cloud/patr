use std::collections::BTreeMap;

use api_models::utils::Uuid;
use k8s_openapi::api::core::v1::Secret;
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

use crate::{migrate_query as query, utils::settings::Settings, Database};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	update_patr_wildcard_certificates(connection, config).await?;
	remove_empty_tags_for_deployments(connection, config).await?;
	update_deployment_table_constraint(connection, config).await?;

	Ok(())
}

async fn update_patr_wildcard_certificates(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	let workspaces = query!(
		r#"
		SELECT
			id
		FROM
			workspace;
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| row.get::<Uuid, _>("id"))
	.collect::<Vec<_>>();
	if workspaces.is_empty() {
		return Ok(());
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
	.await
	.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;
	let client = kube::Client::try_from(kubernetes_config)
		.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;
	let wild_card_secret = Api::<Secret>::namespaced(client.clone(), "default")
		.get("tls-domain-wildcard-patr-cloud")
		.await
		.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;
	let annotations = wild_card_secret
		.metadata
		.annotations
		.ok_or(sqlx::Error::WorkerCrashed)?
		.into_iter()
		.filter(|(key, _)| key.starts_with("cert-manager.io/"))
		.collect::<BTreeMap<String, String>>();
	for workspace in workspaces {
		let workspace_secret =
			Api::<Secret>::namespaced(client.clone(), workspace.as_str())
				.get("tls-domain-wildcard-patr-cloud")
				.await
				.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;
		let mut secret_annotations = workspace_secret
			.metadata
			.annotations
			.ok_or(sqlx::Error::WorkerCrashed)?
			.into_iter()
			.filter(|(key, _)| !key.starts_with("cert-manager.io/"))
			.collect::<BTreeMap<String, String>>();

		secret_annotations.append(&mut annotations.clone());

		let workspace_secret = Secret {
			data: workspace_secret.data,
			immutable: workspace_secret.immutable,
			metadata: ObjectMeta {
				annotations: Some(secret_annotations),
				name: Some("tls-domain-wildcard-patr-cloud".to_string()),
				namespace: Some(workspace.to_string()),
				..ObjectMeta::default()
			},
			..Secret::default()
		};

		Api::<Secret>::namespaced(client.clone(), workspace.as_str())
			.patch(
				"tls-domain-wildcard-patr-cloud",
				&PatchParams::apply("tls-domain-wildcard-patr-cloud").force(),
				&Patch::Apply(workspace_secret),
			)
			.await
			.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;
	}
	Ok(())
}

async fn remove_empty_tags_for_deployments(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			deployment
		SET
			image_tag = 'latest'
		WHERE
			image_tag = '';
	"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn update_deployment_table_constraint(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		ALTER TABLE
			deployment
		ADD CONSTRAINT deployment_chk_image_tag_is_valid 
		CHECK(
			image_tag != ''
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
