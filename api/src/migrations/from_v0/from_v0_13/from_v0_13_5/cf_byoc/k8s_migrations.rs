use std::{collections::HashMap, fmt::Debug};

use api_models::utils::Uuid;
use either::Either;
use futures::{stream, StreamExt, TryStreamExt};
use k8s_openapi::api::{
	core::v1::{Endpoints, Secret, Service},
	networking::v1::{
		HTTPIngressPath,
		HTTPIngressRuleValue,
		Ingress,
		IngressBackend,
		IngressRule,
		IngressServiceBackend,
		IngressSpec,
		ServiceBackendPort,
	},
};
use kube::{
	api::{DeleteParams, PatchParams},
	client::Status,
	config::{
		AuthInfo,
		Cluster,
		Context,
		Kubeconfig,
		NamedAuthInfo,
		NamedCluster,
		NamedContext,
	},
	core::DynamicObject,
	discovery::ApiResource,
	error::ErrorResponse,
	Api,
	Config,
	Error as KubeError,
};
use serde::de::DeserializeOwned;
use sqlx::Row;

use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

#[async_trait::async_trait]
pub trait DeleteOpt<T> {
	async fn delete_opt(
		&self,
		name: &str,
		dp: &DeleteParams,
	) -> kube::Result<Option<Either<T, Status>>>;
}

#[async_trait::async_trait]
impl<T> DeleteOpt<T> for Api<T>
where
	T: Clone + DeserializeOwned + Debug,
{
	async fn delete_opt(
		&self,
		name: &str,
		dp: &DeleteParams,
	) -> kube::Result<Option<Either<T, Status>>> {
		match self.delete(name, dp).await {
			Ok(obj) => Ok(Some(obj)),
			Err(KubeError::Api(ErrorResponse { code: 404, .. })) => Ok(None),
			Err(err) => Err(err),
		}
	}
}

async fn get_kubernetes_client_for_default_region(
	config: &Settings,
) -> Result<kube::Client, Error> {
	let kubeconfig = Config::from_custom_kubeconfig(
		get_default_region_kubeconfig(config),
		&Default::default(),
	)
	.await?;

	let kube_client = kube::Client::try_from(kubeconfig)?;
	Ok(kube_client)
}

pub fn get_default_region_kubeconfig(config: &Settings) -> Kubeconfig {
	Kubeconfig {
		api_version: Some("v1".to_string()),
		kind: Some("Config".to_string()),
		clusters: vec![NamedCluster {
			name: "kubernetesCluster".to_owned(),
			cluster: Cluster {
				server: config.kubernetes.cluster_url.to_owned(),
				certificate_authority_data: Some(
					config.kubernetes.certificate_authority_data.to_owned(),
				),
				insecure_skip_tls_verify: None,
				certificate_authority: None,
				proxy_url: None,
				extensions: None,
			},
		}],
		auth_infos: vec![NamedAuthInfo {
			name: config.kubernetes.auth_username.to_owned(),
			auth_info: AuthInfo {
				token: Some(config.kubernetes.auth_token.to_owned().into()),
				..Default::default()
			},
		}],
		contexts: vec![NamedContext {
			name: "kubernetesContext".to_owned(),
			context: Context {
				cluster: "kubernetesCluster".to_owned(),
				user: config.kubernetes.auth_username.to_owned(),
				extensions: None,
				namespace: None,
			},
		}],
		current_context: Some("kubernetesContext".to_owned()),
		preferences: None,
		extensions: None,
	}
}

pub async fn delete_k8s_static_site_resources(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	let static_site_details = query!(
		r#"
		SELECT
            workspace_id,
			id
		FROM static_site;
		"#
	)
	.fetch(connection)
	.map_ok(|row| {
		(row.get::<Uuid, _>("workspace_id"), row.get::<Uuid, _>("id"))
	})
	.try_collect::<Vec<_>>()
	.await?;

	if static_site_details.is_empty() {
		// added to skip ci error
		return Ok(());
	}

	let kube_client = get_kubernetes_client_for_default_region(config).await?;

	let total_count = static_site_details.len();
	stream::iter(static_site_details)
		.enumerate()
		.map(|(idx, (workspace_id, static_site_id))| {
			let kube_client = kube_client.clone();

			async move {
				let namespace = workspace_id.as_str();

				let svc_deletion_result =
					Api::<Service>::namespaced(kube_client.clone(), namespace)
						.delete_opt(
							&format!("service-{}", static_site_id),
							&DeleteParams::default(),
						)
						.await;
				if let Err(err) = svc_deletion_result {
					return Err((static_site_id, err));
				};

				let ingress_deletion_result =
					Api::<Ingress>::namespaced(kube_client, namespace)
						.delete_opt(
							&format!("ingress-{}", static_site_id),
							&DeleteParams::default(),
						)
						.await;
				if let Err(err) = ingress_deletion_result {
					return Err((static_site_id, err));
				}

				log::info!(
					"{}/{} - Successfully deleted k8s resources for static site {}",
					idx + 1,
					total_count,
					static_site_id
				);

				Result::<Uuid, (Uuid, kube::Error)>::Ok(static_site_id)
			}
		})
		.buffer_unordered(num_cpus::get() * 4)
		.for_each(|task_result| async {
			match task_result {
				Ok(_) => {}
				Err((static_site_id, err)) => log::info!(
					"Error while deleting static site resource {} - {}",
					static_site_id,
					err
				),
			}
		})
		.await;

	Ok(())
}

pub async fn delete_k8s_region_resources(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	let region_details = query!(
		r#"
		SELECT
            workspace_id,
			id
		FROM region
		WHERE
			workspace_id IS NOT NULL;
		"#
	)
	.fetch(connection)
	.map_ok(|row| {
		(row.get::<Uuid, _>("workspace_id"), row.get::<Uuid, _>("id"))
	})
	.try_collect::<Vec<_>>()
	.await?;

	if region_details.is_empty() {
		// added to skip ci error
		return Ok(());
	}

	let kube_client = get_kubernetes_client_for_default_region(config).await?;

	let total_count = region_details.len();
	stream::iter(region_details)
		.enumerate()
		.map(|(idx, (workspace_id, region_id))| {
			let kube_client = kube_client.clone();

			async move {
				let namespace = workspace_id.as_str();

				let svc_deletion_result =
					Api::<Service>::namespaced(kube_client.clone(), namespace)
						.delete_opt(
							&format!("service-{}", region_id),
							&DeleteParams::default(),
						)
						.await;
				if let Err(err) = svc_deletion_result {
					return Err((region_id, err));
				};

				let endpoint_deletion_result =
					Api::<Endpoints>::namespaced(kube_client, namespace)
						.delete_opt(
							&format!("service-{}", region_id),
							&DeleteParams::default(),
						)
						.await;
				if let Err(err) = endpoint_deletion_result {
					return Err((region_id, err));
				}

				log::info!(
					"{}/{} - Successfully deleted k8s resources for region {}",
					idx + 1,
					total_count,
					region_id
				);

				Result::<Uuid, (Uuid, kube::Error)>::Ok(region_id)
			}
		})
		.buffer_unordered(num_cpus::get() * 4)
		.for_each(|task_result| async {
			match task_result {
				Ok(_) => {}
				Err((static_site_id, err)) => log::info!(
					"Error while deleting region resource {} - {}",
					static_site_id,
					err
				),
			}
		})
		.await;

	Ok(())
}

pub async fn delete_k8s_managed_url_resources(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	let managed_url_details = query!(
		r#"
		SELECT
            workspace_id,
			id
		FROM managed_url;
		"#
	)
	.fetch(connection)
	.map_ok(|row| {
		(row.get::<Uuid, _>("workspace_id"), row.get::<Uuid, _>("id"))
	})
	.try_collect::<Vec<_>>()
	.await?;

	if managed_url_details.is_empty() {
		// added to skip ci error
		return Ok(());
	}

	let kube_client = get_kubernetes_client_for_default_region(config).await?;

	let total_count = managed_url_details.len();
	stream::iter(managed_url_details)
		.enumerate()
		.map(|(idx, (workspace_id, managed_url_id))| {
			let kube_client = kube_client.clone();

			async move {
				let namespace = workspace_id.as_str();

				let svc_deletion_result =
					Api::<Service>::namespaced(kube_client.clone(), namespace)
						.delete_opt(
							&format!("service-{}", managed_url_id),
							&DeleteParams::default(),
						)
						.await;
				if let Err(err) = svc_deletion_result {
					return Err((managed_url_id, err));
				};

				let ingress_deletion_result = Api::<Ingress>::namespaced(
					kube_client.clone(),
					namespace,
				)
				.delete_opt(
					&format!("ingress-{}", managed_url_id),
					&DeleteParams::default(),
				)
				.await;
				if let Err(err) = ingress_deletion_result {
					return Err((managed_url_id, err));
				}

				let svc_verification_deletion_result =
					Api::<Service>::namespaced(kube_client.clone(), namespace)
						.delete_opt(
							&format!("service-{}-verification", managed_url_id),
							&DeleteParams::default(),
						)
						.await;
				if let Err(err) = svc_verification_deletion_result {
					return Err((managed_url_id, err));
				}

				let ingress_verification_deletion_result =
					Api::<Ingress>::namespaced(kube_client, namespace)
						.delete_opt(
							&format!("ingress-{}-verification", managed_url_id),
							&DeleteParams::default(),
						)
						.await;
				if let Err(err) = ingress_verification_deletion_result {
					return Err((managed_url_id, err));
				}

				log::info!(
					"{}/{} - Successfully deleted k8s resources for region {}",
					idx + 1,
					total_count,
					managed_url_id
				);

				Result::<Uuid, (Uuid, kube::Error)>::Ok(managed_url_id)
			}
		})
		.buffer_unordered(num_cpus::get() * 4)
		.for_each(|task_result| async {
			match task_result {
				Ok(_) => {}
				Err((static_site_id, err)) => log::info!(
					"Error while deleting region resource {} - {}",
					static_site_id,
					err
				),
			}
		})
		.await;

	Ok(())
}

pub async fn delete_k8s_certificate_resources_for_domain(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	let domain_details = query!(
		r#"
		SELECT
            resource.owner_id,
			workspace_domain.id
		FROM workspace_domain
		JOIN resource
			ON resource.id = workspace_domain.id;
		"#
	)
	.fetch(&mut *connection)
	.map_ok(|row| (row.get::<Uuid, _>("owner_id"), row.get::<Uuid, _>("id")))
	.try_collect::<Vec<_>>()
	.await?;

	if domain_details.is_empty() {
		// added to skip ci error
		return Ok(());
	}

	let kube_client = get_kubernetes_client_for_default_region(config).await?;

	let total_count = domain_details.len();
	stream::iter(domain_details)
		.enumerate()
		.map(|(idx, (workspace_id, domain_id))| {
			let kube_client = kube_client.clone();

			async move {
				let namespace = workspace_id.as_str();

				let certificate_name = format!("certificate-{}", domain_id);
				let certificate_resource = ApiResource {
					group: "cert-manager.io".to_string(),
					version: "v1".to_string(),
					api_version: "cert-manager.io/v1".to_string(),
					kind: "certificate".to_string(),
					plural: "certificates".to_string(),
				};
				let cert_deletion_result =
					Api::<DynamicObject>::namespaced_with(
						kube_client.clone(),
						namespace,
						&certificate_resource,
					)
					.delete_opt(&certificate_name, &DeleteParams::default())
					.await;
				if let Err(err) = cert_deletion_result {
					return Err((domain_id, err));
				};

				let secret_name = format!("tls-{}", domain_id);
				let secret_deletion_result =
					Api::<Secret>::namespaced(kube_client.clone(), namespace)
						.delete_opt(&secret_name, &DeleteParams::default())
						.await;
				if let Err(err) = secret_deletion_result {
					return Err((domain_id, err));
				}

				log::info!(
					    "{}/{} - Successfully deleted k8s certificate resources for domain {}",
					    idx + 1,
					    total_count,
					    domain_id
				    );

				Result::<Uuid, (Uuid, kube::Error)>::Ok(domain_id)
			}
		})
		.buffer_unordered(num_cpus::get() * 4)
		.for_each(|task_result| async {
			match task_result {
				Ok(_) => {}
				Err((domain_id, err)) => log::info!(
					    "Error while deleting certificate resource for domain {} - {}",
					    domain_id,
					    err
				    ),
			}
		})
		.await;
	Ok(())
}

pub async fn delete_k8s_certificate_resources_for_managed_url(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	let managed_url_domain_details = query!(
		r#"
		SELECT
			id,
			workspace_id,
			sub_domain,
			domain_id
		FROM managed_url
		WHERE sub_domain != '@';
		"#
	)
	.fetch(connection)
	.map_ok(|row| {
		(
			row.get::<Uuid, _>("id"),
			row.get::<Uuid, _>("workspace_id"),
			row.get::<String, _>("sub_domain"),
			row.get::<Uuid, _>("domain_id"),
		)
	})
	.try_collect::<Vec<_>>()
	.await?;

	if managed_url_domain_details.is_empty() {
		// added to skip ci error
		return Ok(());
	}

	let kube_client = get_kubernetes_client_for_default_region(config).await?;

	let total_count = managed_url_domain_details.len();
	stream::iter(managed_url_domain_details)
		.enumerate()
		.map(
			|(idx, (managed_url_id, workspace_id, sub_domain, domain_id))| {
				let kube_client = kube_client.clone();

				async move {
					let namespace = workspace_id.as_str();

					let certificate_name =
						format!("certificate-{}-{}", sub_domain, domain_id);
					let certificate_resource = ApiResource {
						group: "cert-manager.io".to_string(),
						version: "v1".to_string(),
						api_version: "cert-manager.io/v1".to_string(),
						kind: "certificate".to_string(),
						plural: "certificates".to_string(),
					};
					let cert_deletion_result =
						Api::<DynamicObject>::namespaced_with(
							kube_client.clone(),
							namespace,
							&certificate_resource,
						)
						.delete_opt(&certificate_name, &DeleteParams::default())
						.await;
					if let Err(err) = cert_deletion_result {
						return Err((managed_url_id, err));
					};

					let certificate_name = format!(
						"certificate-{}-{}",
						sub_domain, managed_url_id
					);
					let certificate_resource = ApiResource {
						group: "cert-manager.io".to_string(),
						version: "v1".to_string(),
						api_version: "cert-manager.io/v1".to_string(),
						kind: "certificate".to_string(),
						plural: "certificates".to_string(),
					};
					let cert_deletion_result =
						Api::<DynamicObject>::namespaced_with(
							kube_client.clone(),
							namespace,
							&certificate_resource,
						)
						.delete_opt(&certificate_name, &DeleteParams::default())
						.await;
					if let Err(err) = cert_deletion_result {
						return Err((managed_url_id, err));
					};

					let secret_name =
						format!("tls-{}-{}", sub_domain, domain_id);
					let secret_deletion_result = Api::<Secret>::namespaced(
						kube_client.clone(),
						namespace,
					)
					.delete_opt(&secret_name, &DeleteParams::default())
					.await;
					if let Err(err) = secret_deletion_result {
						return Err((managed_url_id, err));
					}

					let secret_name =
						format!("tls-{}-{}", sub_domain, managed_url_id);
					let secret_deletion_result = Api::<Secret>::namespaced(
						kube_client.clone(),
						namespace,
					)
					.delete_opt(&secret_name, &DeleteParams::default())
					.await;
					if let Err(err) = secret_deletion_result {
						return Err((managed_url_id, err));
					}

					log::info!(
						    "{}/{} - Successfully deleted k8s certificate resources for managed_url {}",
						    idx + 1,
						    total_count,
						    managed_url_id
					    );

					Result::<Uuid, (Uuid, kube::Error)>::Ok(managed_url_id)
				}
			},
		)
		.buffer_unordered(num_cpus::get() * 4)
		.for_each(|task_result| async {
			match task_result {
				Ok(_) => {}
				Err((domain_id, err)) => log::info!(
						    "Error while deleting certificate resource for managed_url {} - {}",
						    domain_id,
						    err
					    ),
			}
		})
		.await;
	Ok(())
}

pub async fn patch_ingress_for_default_region_deployments(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	let default_region_id = query!(
		r#"
		SELECT
			id
		FROM
			region
		WHERE
			name = 'Singapore'
			AND provider = 'digitalocean'
			AND workspace_id IS NULL
			AND status = 'active';
		"#
	)
	.fetch_one(&mut *connection)
	.await
	.map(|row| row.get::<Uuid, _>("id"))
	.expect("Default region should be present already");

	let running_deployments = query!(
		r#"
		SELECT
			deployment.workspace_id,
			deployment.id as "deployment_id",
			deployment_exposed_port.port
		FROM
			deployment
		JOIN deployment_exposed_port
			ON deployment_exposed_port.deployment_id = deployment.id
		WHERE
			deployment.status = 'running' AND
			deployment.deleted IS NULL;
		"#
	)
	.fetch(&mut *connection)
	.map_ok(|row| {
		(
			row.get::<Uuid, _>("workspace_id"),
			row.get::<Uuid, _>("deployment_id"),
			row.get::<i32, _>("port"),
		)
	})
	.try_collect::<Vec<_>>()
	.await?;

	let running_deployments = running_deployments.into_iter().fold(
		HashMap::<(Uuid, Uuid), Vec<i32>>::new(),
		|mut accu, (workspace_id, deployment_id, port)| {
			accu.entry((workspace_id, deployment_id))
				.or_default()
				.push(port);
			accu
		},
	);

	if running_deployments.is_empty() {
		// added to skip ci error
		return Ok(());
	}

	let kube_client = get_kubernetes_client_for_default_region(config).await?;

	let total_count = running_deployments.len();
	stream::iter(running_deployments)
		.enumerate()
		.map(|(idx, ((workspace_id, deployment_id), ports))| {
			let kube_client = kube_client.clone();
			let default_region_id = &default_region_id;
			async move {
				let namespace = workspace_id.as_str();
				let ingress_name =
					format!("ingress-{}", deployment_id.as_str());

				let annotations = [(
					"kubernetes.io/ingress.class".to_string(),
					"nginx".to_string(),
				)]
				.into();

				let ingress_rules = ports
					.into_iter()
					.map(|port| IngressRule {
						host: Some(format!(
							"{}-{}.{}.{}",
							port,
							deployment_id,
							default_region_id,
							config.cloudflare.onpatr_domain
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
											number: Some(port),
											..Default::default()
										}),
									}),
									..Default::default()
								},
								path: Some("/".to_string()),
								path_type: Some("Prefix".to_string()),
							}],
						}),
					})
					.collect();

				let patch_request = Ingress {
					metadata: kube::core::ObjectMeta {
						annotations: Some(annotations),
						..Default::default()
					},
					spec: Some(IngressSpec {
						rules: Some(ingress_rules),
						..Default::default()
					}),
					..Default::default()
				};

				let result = Api::<Ingress>::namespaced(kube_client, namespace)
					.patch(
						&ingress_name,
						&PatchParams::default(),
						&kube::api::Patch::Strategic(patch_request),
					)
					.await;

				match result {
					Ok(_ingress) => {
						log::info!(
							"{}/{} - Successfully patched ingress for deployment {}",
							idx + 1,
							total_count,
							deployment_id
						);
					}
					Err(kube::Error::Api(ErrorResponse {
						code: 404, ..
					})) => log::error!(
						"{}/{} - Ingress for deployment {} not found, hence skipping patch",
						idx + 1,
						total_count,
						deployment_id
					),
					Err(err) => return Err((deployment_id, err)),
				}

				Result::<Uuid, (Uuid, kube::Error)>::Ok(deployment_id)
			}
		})
		.buffer_unordered(num_cpus::get() * 4)
		.for_each(|task_result| async {
			match task_result {
				Ok(_) => {}
				Err((deployment_id, err)) => log::info!(
					"Error while patching ingress for deployment {} - {}",
					deployment_id,
					err
				),
			}
		})
		.await;

	Ok(())
}
