use std::{collections::BTreeMap, ops::DerefMut};

use eve_rs::AsError;
use k8s_openapi::{
	api::{
		apps::v1::{Deployment, DeploymentSpec},
		core::v1::{
			Container,
			ContainerPort,
			LocalObjectReference,
			PodSpec,
			PodTemplateSpec,
			Service,
			ServicePort,
			ServiceSpec,
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
	apimachinery::pkg::{
		apis::meta::v1::LabelSelector,
		util::intstr::IntOrString,
	},
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
use uuid::Uuid;

use crate::{
	db,
	error,
	models::db_mapping::DeploymentStatus,
	service,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn update_deployment(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &[u8],
	config: Settings,
) -> Result<(), Error> {
	let kubernetes_client = get_kubernetes_config(&config).await?;

	let request_id = Uuid::new_v4();
	// TODO: remove this once DTO part is complete
	let deployment = db::get_deployment_by_id(
		service::get_app().database.acquire().await?.deref_mut(),
		&deployment_id,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	log::trace!("Deploying the container with id: {} and image: {:?} on DigitalOcean Managed Kubernetes with request_id: {}",
		hex::encode(&deployment_id),
		deployment.get_full_image(connection).await?,
		request_id,
	);

	let deployment_id_string = hex::encode(&deployment_id);
	// new name for the docker image
	let new_repo_name = format!(
		"registry.digitalocean.com/{}/{}",
		config.digitalocean.registry, deployment_id_string,
	);
	let horizontal_scale = deployment.horizontal_scale as i32;

	log::trace!(
		"request_id: {} - Deploying deployment: {}",
		request_id,
		deployment_id_string,
	);
	let _ = super::update_deployment_status(
		&deployment_id,
		&DeploymentStatus::Pushed,
	)
	.await;

	// TODO: change the namespace to workspace id
	let namespace = "default";

	let mut labels: BTreeMap<String, String> = BTreeMap::new();
	labels.insert("app".to_owned(), deployment_id_string.clone());

	log::trace!(
		"request_id: {} - generating deployment configuration",
		request_id
	);

	let _ = super::update_deployment_status(
		&deployment_id,
		&DeploymentStatus::Deploying,
	)
	.await;

	let kubernetes_deployment = Deployment {
		metadata: ObjectMeta {
			name: Some(deployment_id_string.to_string()),
			namespace: Some(namespace.to_string()),
			labels: Some(labels.clone()),
			..ObjectMeta::default()
		},
		spec: Some(DeploymentSpec {
			replicas: Some(horizontal_scale),
			selector: LabelSelector {
				match_expressions: None,
				match_labels: Some(labels.clone()),
			},
			template: PodTemplateSpec {
				spec: Some(PodSpec {
					containers: vec![Container {
						name: deployment_id_string.to_string(),
						image: Some(new_repo_name.to_string()),
						ports: Some(vec![ContainerPort {
							container_port: 80,
							name: Some("http".to_owned()),
							..ContainerPort::default()
						}]),
						..Container::default()
					}],
					image_pull_secrets: Some(vec![LocalObjectReference {
						name: Some("regcred".to_string()),
					}]),
					..PodSpec::default()
				}),
				metadata: Some(ObjectMeta {
					labels: Some(labels.clone()),
					..ObjectMeta::default()
				}),
			},
			..DeploymentSpec::default()
		}),
		..Deployment::default()
	};

	// Create the deployment defined above
	log::trace!("request_id: {} - creating deployment", request_id);
	let deployment_api =
		Api::<Deployment>::namespaced(kubernetes_client.clone(), namespace);

	deployment_api
		.patch(
			&deployment_id_string,
			&PatchParams::apply(&deployment_id_string),
			&Patch::Apply(kubernetes_deployment),
		)
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let kubernetes_service = Service {
		metadata: ObjectMeta {
			name: Some(format!("service-{}", &deployment_id_string)),
			..ObjectMeta::default()
		},
		spec: Some(ServiceSpec {
			ports: Some(vec![ServicePort {
				port: 80,
				target_port: Some(IntOrString::Int(80)),
				name: Some("http".to_owned()),
				..ServicePort::default()
			}]),
			selector: Some(labels),
			..ServiceSpec::default()
		}),
		..Service::default()
	};

	// Create the service defined above
	log::trace!("request_id: {} - creating ClusterIp service", request_id);
	let service_api: Api<Service> =
		Api::namespaced(kubernetes_client.clone(), namespace);

	service_api
		.patch(
			&format!("service-{}", &deployment_id_string),
			&PatchParams::apply(&format!("service-{}", &deployment_id_string)),
			&Patch::Apply(kubernetes_service),
		)
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let mut annotations: BTreeMap<String, String> = BTreeMap::new();
	annotations.insert(
		"kubernetes.io/ingress.class".to_string(),
		"nginx".to_string(),
	);

	log::trace!(
		"request_id: {} - creating https certificates for domain",
		request_id
	);

	annotations.insert(
		"cert-manager.io/issuer".to_string(),
		"letsencrypt-prod".to_string(),
	);

	let custom_domain_rule = if let Some(domain) =
		deployment.domain_name.clone()
	{
		log::trace!("request_id: {} - custom domain present, adding domain details to the ingress", request_id);
		annotations.insert(
			"nginx.ingress.kubernetes.io/proxy-redirect-from".to_string(),
			domain.clone(),
		);

		annotations.insert(
			"nginx.ingress.kubernetes.io/proxy-redirect-to".to_string(),
			format!("{}.patr.cloud", deployment_id_string),
		);

		vec![
			IngressRule {
				host: Some(format!("{}.patr.cloud", deployment_id_string)),
				http: Some(HTTPIngressRuleValue {
					paths: vec![HTTPIngressPath {
						backend: IngressBackend {
							service: Some(IngressServiceBackend {
								name: format!(
									"service-{}",
									&deployment_id_string
								),
								port: Some(ServiceBackendPort {
									number: Some(80),
									name: Some("http".to_owned()),
								}),
							}),
							..IngressBackend::default()
						},
						..HTTPIngressPath::default()
					}],
				}),
			},
			IngressRule {
				host: Some(domain),
				http: Some(HTTPIngressRuleValue {
					paths: vec![HTTPIngressPath {
						backend: IngressBackend {
							service: Some(IngressServiceBackend {
								name: format!(
									"service-{}",
									&deployment_id_string
								),
								port: Some(ServiceBackendPort {
									number: Some(80),
									name: Some("http".to_owned()),
								}),
							}),
							..IngressBackend::default()
						},
						..HTTPIngressPath::default()
					}],
				}),
			},
		]
	} else {
		vec![IngressRule {
			host: Some(format!("{}.patr.cloud", deployment_id_string)),
			http: Some(HTTPIngressRuleValue {
				paths: vec![HTTPIngressPath {
					backend: IngressBackend {
						service: Some(IngressServiceBackend {
							name: format!("service-{}", &deployment_id_string),
							port: Some(ServiceBackendPort {
								number: Some(80),
								name: Some("http".to_owned()),
							}),
						}),
						..IngressBackend::default()
					},
					..HTTPIngressPath::default()
				}],
			}),
		}]
	};

	let custom_domain_tls = if let Some(domain) = deployment.domain_name {
		log::trace!(
			"request_id: {} - adding custom domain config to ingress",
			request_id
		);
		vec![
			IngressTLS {
				hosts: Some(vec![format!(
					"{}.patr.cloud",
					deployment_id_string
				)]),
				secret_name: Some(format!("tls-{}", &deployment_id_string)),
			},
			IngressTLS {
				hosts: Some(vec![domain.to_string()]),
				secret_name: Some(format!(
					"custom-tls-{}",
					&deployment_id_string
				)),
			},
		]
	} else {
		log::trace!(
			"request_id: {} - adding patr domain config to ingress",
			request_id
		);
		vec![IngressTLS {
			hosts: Some(vec![format!("{}.patr.cloud", deployment_id_string)]),
			secret_name: Some(format!("tls-{}", &deployment_id_string)),
		}]
	};

	let kubernetes_ingress: Ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(format!("ingress-{}", &deployment_id_string)),
			annotations: Some(annotations),
			..ObjectMeta::default()
		},
		spec: Some(IngressSpec {
			rules: Some(custom_domain_rule),
			tls: Some(custom_domain_tls),
			..IngressSpec::default()
		}),
		..Ingress::default()
	};

	// Create the ingress defined above
	log::trace!("request_id: {} - creating ingress", request_id);
	let ingress_api: Api<Ingress> =
		Api::namespaced(kubernetes_client, namespace);

	ingress_api
		.patch(
			&format!("ingress-{}", &deployment_id_string),
			&PatchParams::apply(&format!("ingress-{}", &deployment_id_string)),
			&Patch::Apply(kubernetes_ingress),
		)
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	log::trace!("request_id: {} - deployment created", request_id);

	log::trace!(
		"request_id: {} - App ingress is at {}.patr.cloud",
		request_id,
		deployment_id_string
	);

	Ok(())
}

async fn get_kubernetes_config(
	config: &Settings,
) -> Result<kube::Client, Error> {
	let config = Config::from_custom_kubeconfig(
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
	.await?;

	let client = kube::Client::try_from(config)?;

	Ok(client)
}
