use std::collections::BTreeMap;

use api_models::utils::Uuid;
use k8s_openapi::api::networking::v1::{
	HTTPIngressPath,
	HTTPIngressRuleValue,
	Ingress,
	IngressBackend,
	IngressRule,
	IngressServiceBackend,
	IngressSpec,
	IngressTLS,
	ServiceBackendPort,
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
									path_type: "Prefix".to_string(),
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
		.await?;
	}

	Ok(())
}
