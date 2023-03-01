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
							path_type: "Prefix".to_string(),
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
		.await?;
	}

	Ok(())
}
