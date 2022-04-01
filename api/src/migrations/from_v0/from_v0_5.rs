use std::collections::BTreeMap;

use api_models::utils::Uuid;
use k8s_openapi::{
	api::{
		apps::v1::{Deployment, DeploymentSpec},
		core::v1::{
			Container,
			LocalObjectReference,
			PodSpec,
			PodTemplateSpec,
			Secret,
		},
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
	},
	apimachinery::pkg::apis::meta::v1::LabelSelector,
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
use reqwest::Client;
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
		(0, 5, 6) => migrate_from_v0_5_6(&mut *connection, config).await?,
		(0, 5, 7) => migrate_from_v0_5_7(&mut *connection, config).await?,
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
	vec![
		"0.5.0", "0.5.1", "0.5.2", "0.5.3", "0.5.4", "0.5.5", "0.5.6", "0.5.7",
	]
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
			port_type = 'http' AND
			deployment.status = 'running';
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

async fn migrate_from_v0_5_6(
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

	let static_site_list = query!(
		r#"
		SELECT
			id,
			workspace_id
		FROM
			deployment_static_site
		WHERE
			status = 'running';
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.get::<Uuid, _>("id"), row.get::<Uuid, _>("workspace_id")));

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

	let kubernetes_client = kube::Client::try_from(kubernetes_config)
		.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;

	let ingress_tls_rules = IngressTLS {
		hosts: Some(vec!["*.patr.cloud".to_string(), "patr.cloud".to_string()]),
		secret_name: None,
	};

	for (static_site_id, workspace_id) in static_site_list {
		let annotations = [
			(
				"kubernetes.io/ingress.class".to_string(),
				"nginx".to_string(),
			),
			(
				"nginx.ingress.kubernetes.io/upstream-vhost".to_string(),
				format!("{}.patr.cloud", static_site_id),
			),
			(
				"cert-manager.io/cluster-issuer".to_string(),
				config.kubernetes.cert_issuer_dns.clone(),
			),
		]
		.into_iter()
		.collect::<BTreeMap<_, _>>();

		let kubernetes_ingress = Ingress {
			metadata: ObjectMeta {
				name: Some(format!("ingress-{}", static_site_id)),
				annotations: Some(annotations),
				..ObjectMeta::default()
			},
			spec: Some(IngressSpec {
				rules: Some(vec![IngressRule {
					host: Some(format!("{}.patr.cloud", static_site_id)),
					http: Some(HTTPIngressRuleValue {
						paths: vec![HTTPIngressPath {
							backend: IngressBackend {
								service: Some(IngressServiceBackend {
									name: format!("service-{}", static_site_id),
									port: Some(ServiceBackendPort {
										number: Some(80),
										..ServiceBackendPort::default()
									}),
								}),
								..Default::default()
							},
							path: Some("/".to_string()),
							path_type: Some("Prefix".to_string()),
						}],
					}),
				}]),
				tls: Some(vec![ingress_tls_rules.clone()]),
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
			&format!("ingress-{}", static_site_id),
			&PatchParams::apply(&format!("ingress-{}", static_site_id)),
			&Patch::Apply(kubernetes_ingress),
		)
		.await
		.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;
	}

	Ok(())
}

async fn migrate_from_v0_5_7(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	rabbitmq(connection, config).await?;
	audit_logs(connection).await?;
	chargebee(connection, config).await?;
	rename_backup_email_to_recovery_email(connection).await?;

	Ok(())
}

async fn rabbitmq(
	connection: &mut sqlx::PgConnection,
	config: &Settings,
) -> Result<(), sqlx::Error> {
	let deployment_list = query!(
		r#"
		SELECT
			deployment.id,
			deployment.workspace_id,
			workspace.name,
			docker_registry_repository.name as "repository",
			deployment.image_tag,
			deployment.region
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
			row.get::<Uuid, _>("workspace_id"),
			row.get::<String, _>("name"),
			row.get::<String, _>("repository"),
			row.get::<String, _>("image_tag"),
			row.get::<String, _>("region"),
		)
	})
	.collect::<Vec<_>>();
	if deployment_list.is_empty() {
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

	for (
		deployment_id,
		workspace_id,
		workspace_name,
		repository,
		image_tag,
		region,
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
			spec: Some(DeploymentSpec {
				selector: LabelSelector {
					match_labels: Some(labels.clone()),
					..LabelSelector::default()
				},
				template: PodTemplateSpec {
					metadata: Some(ObjectMeta {
						labels: Some(labels),
						..ObjectMeta::default()
					}),
					spec: Some(PodSpec {
						containers: vec![Container {
							image: Some(format!(
								"registry.patr.cloud/{}/{}:{}",
								workspace_name, repository, image_tag
							)),
							..Container::default()
						}],
						image_pull_secrets: Some(vec![LocalObjectReference {
							name: Some("patr-regcred".to_string()),
						}]),
						..PodSpec::default()
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
			.await
			.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;
	}
	Ok(())
}

async fn audit_logs(
	connection: &mut sqlx::PgConnection,
) -> Result<(), sqlx::Error> {
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
) -> Result<(), sqlx::Error> {
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
				id=$1;
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
			.await
			.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;

		client
			.post(format!("{}/promotional_credits/set", config.chargebee.url))
			.basic_auth(config.chargebee.api_key.as_str(), password.as_ref())
			.query(&[
				("customer_id", workspace_id.as_str()),
				("amount", &config.chargebee.credit_amount),
				("description", &config.chargebee.description),
			])
			.send()
			.await
			.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;

		let deployments = query!(
			r#"
			SELECT
				id,
				min_horizontal_scale,
				machine_type,
			FROM
				deployment
			WHERE
				workspace_id=$1 AND
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
				row.get::<String, _>("machine_type"),
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
						machine_type.to_string(),
					),
					(
						"subscription_items[quantity][0]",
						min_horizontal_scale.to_string(),
					),
				])
				.send()
				.await
				.map_err(|err| sqlx::Error::Configuration(Box::new(err)))?;
		}
	}

	Ok(())
}

async fn rename_backup_email_to_recovery_email(
	connection: &mut sqlx::PgConnection,
) -> Result<(), sqlx::Error> {
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
