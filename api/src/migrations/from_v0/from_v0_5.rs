use std::collections::BTreeMap;

use api_models::utils::Uuid;
use k8s_openapi::api::{
	core::v1::Secret,
	networking::v1::{
		HTTPIngressPath,
		HTTPIngressRuleValue,
		Ingress,
		IngressBackend,
		IngressRule,
		IngressServiceBackend,
		IngressSpec,
		IngressTLS,
		ServiceBackendPort,
	},
};
use kube::{
	api::{ListParams, Patch, PatchParams},
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
use semver::Version;
use sqlx::Row;

use crate::{migrate_query as query, utils::settings::Settings, Database};

/// # Description
/// The function is used to migrate the database from one version to another
///
/// # Arguments
/// * `connection` - database save point, more details here: [`Transaction`]
/// * `version` - A struct containing the version to upgrade from. Panics if the
///   version is not 0.x.x, more info here: [`Version`]: Version
///
/// # Return
/// This function returns Result<(), Error> containing an empty response or
/// sqlx::error
///
/// [`Constants`]: api/src/utils/constants.rs
/// [`Transaction`]: Transaction
pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	version: Version,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	match (version.major, version.minor, version.patch) {
		(0, 5, 0) => migrate_from_v0_5_0(&mut *connection, config).await?,
		(0, 5, 1) => migrate_from_v0_5_1(&mut *connection, config).await?,
		(0, 5, 2) => migrate_from_v0_5_2(&mut *connection, config).await?,
		(0, 5, 3) => migrate_from_v0_5_3(&mut *connection, config).await?,
		(0, 5, 4) => migrate_from_v0_5_4(&mut *connection, config).await?,
		(0, 5, 5) => migrate_from_v0_5_5(&mut *connection, config).await?,
		_ => {
			panic!("Migration from version {} is not implemented yet!", version)
		}
	}

	Ok(())
}

/// # Description
/// The function is used to get a list of all 0.3.x migrations to migrate the
/// database from
///
/// # Return
/// This function returns [&'static str; _] containing a list of all migration
/// versions
pub fn get_migrations() -> Vec<&'static str> {
	vec!["0.5.0", "0.5.1", "0.5.2", "0.5.3", "0.5.4", "0.5.5"]
}

async fn migrate_from_v0_5_0(
	_connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	Ok(())
}

async fn migrate_from_v0_5_1(
	_connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	Ok(())
}

async fn migrate_from_v0_5_2(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	update_patr_wildcard_certificates(connection, config).await?;
	remove_empty_tags_for_deployments(connection).await?;
	update_deployment_table_constraint(connection).await?;

	Ok(())
}

async fn migrate_from_v0_5_3(
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
	.await?;

	if workspaces
		.into_iter()
		.map(|row| row.get::<Uuid, _>("id"))
		.next()
		.is_none()
	{
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
					token: Some(config.kubernetes.auth_token.clone()),
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

	let ingress_list = Api::<Ingress>::all(client.clone())
		.list(&ListParams::default())
		.await
		.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;

	for ingress in ingress_list {
		let (default_backend, ingress_class_name, rules) =
			if let Some(spec) = ingress.spec.clone() {
				(spec.default_backend, spec.ingress_class_name, spec.rules)
			} else {
				(None, None, None)
			};

		let (name, namespace) = if let (Some(name), Some(namespace)) = (
			ingress.metadata.name.clone(),
			ingress.metadata.namespace.clone(),
		) {
			(name, namespace)
		} else {
			continue;
		};

		let ingress = Ingress {
			metadata: ObjectMeta {
				name: ingress.metadata.name,
				annotations: ingress.metadata.annotations,
				..ObjectMeta::default()
			},
			spec: Some(IngressSpec {
				default_backend,
				ingress_class_name,
				rules,
				tls: Some(vec![IngressTLS {
					hosts: Some(vec![
						"*.patr.cloud".to_string(),
						"patr.cloud".to_string(),
					]),
					secret_name: Some(
						"tls-domain-wildcard-patr-cloud".to_string(),
					),
				}]),
			}),
			..Ingress::default()
		};

		let _ = Api::<Ingress>::namespaced(client.clone(), &namespace)
			.patch(
				&name,
				&PatchParams::apply(&name).force(),
				&Patch::Apply(ingress),
			)
			.await
			.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;
	}

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
					token: Some(config.kubernetes.auth_token.clone()),
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

async fn migrate_from_v0_5_4(
	_connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	Ok(())
}

async fn migrate_from_v0_5_5(
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
	.await?;

	if workspaces.is_empty() {
		return Ok(());
	}

	let deployment_port_list = query!(
		r#"
		SELECT
			deployment_exposed_port.deployment_id,
			deployment_exposed_port.port,
			deployment.workspace_id
		FROM
			deployment_exposed_port
		INNER JOIN
			deployment
		ON
			deployment.id = deployment_exposed_port.deployment_id
		WHERE
			port_type = 'http';
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| {
		(
			row.get::<Uuid, _>("deployment_id"),
			row.get::<Uuid, _>("workspace_id"),
			row.get::<i32, _>("port") as u16,
		)
	})
	.collect::<Vec<_>>();

	let mut deployment_ports = BTreeMap::new();

	for (deployment_id, workspace_id, port) in deployment_port_list {
		deployment_ports
			.entry((deployment_id, workspace_id))
			.or_insert_with(Vec::new)
			.push(port);
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
					token: Some(config.kubernetes.auth_token.clone()),
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

	let kubernetes_client = kube::Client::try_from(kubernetes_config)
		.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;

	let annotations = [
		(
			"kubernetes.io/ingress.class".to_string(),
			"nginx".to_string(),
		),
		(
			"cert-manager.io/cluster-issuer".to_string(),
			config.kubernetes.cert_issuer_dns.clone(),
		),
	]
	.into_iter()
	.collect::<BTreeMap<_, _>>();

	let ingress_tls_rules = IngressTLS {
		hosts: Some(vec!["*.patr.cloud".to_string(), "patr.cloud".to_string()]),
		secret_name: None,
	};

	for ((deployment_id, workspace_id), ports) in deployment_ports {
		let kubernetes_ingress = Ingress {
			metadata: ObjectMeta {
				name: Some(format!("ingress-{}", deployment_id)),
				annotations: Some(annotations.clone()),
				..ObjectMeta::default()
			},
			spec: Some(IngressSpec {
				rules: Some(
					ports
						.iter()
						.map(|port| IngressRule {
							host: Some(format!(
								"{}-{}.patr.cloud",
								port, deployment_id
							)),
							http: Some(HTTPIngressRuleValue {
								paths: vec![HTTPIngressPath {
									backend: IngressBackend {
										service: Some(IngressServiceBackend {
											name: format!(
												"service-{}",
												deployment_id
											),
											port: Some(ServiceBackendPort {
												number: Some(*port as i32),
												..ServiceBackendPort::default()
											}),
										}),
										..Default::default()
									},
									path: Some("/".to_string()),
									path_type: Some("Prefix".to_string()),
								}],
							}),
						})
						.collect(),
				),
				tls: Some(
					ports.iter().map(|_| ingress_tls_rules.clone()).collect(),
				),
				..IngressSpec::default()
			}),
			..Ingress::default()
		};

		// Create the ingress defined above
		Api::<Ingress>::namespaced(
			kubernetes_client.clone(),
			workspace_id.as_str(),
		)
		.patch(
			&format!("ingress-{}", deployment_id),
			&PatchParams::apply(&format!("ingress-{}", deployment_id)),
			&Patch::Apply(kubernetes_ingress),
		)
		.await
		.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;
	}

	Ok(())
}
