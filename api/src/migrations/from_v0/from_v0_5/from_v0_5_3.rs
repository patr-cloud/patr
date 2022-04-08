use api_models::utils::Uuid;
use k8s_openapi::api::networking::v1::{Ingress, IngressSpec, IngressTLS};
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
use sqlx::Row;

use crate::{migrate_query as query, utils::settings::Settings, Database};

pub(super) async fn migrate(
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
