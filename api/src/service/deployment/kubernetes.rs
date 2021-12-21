use std::collections::BTreeMap;

use api_models::{
	models::workspace::infrastructure::deployment::{
		DeploymentStatus,
		EntryPointMapping,
		EnvironmentVariableValue,
		ExposedPortType,
	},
	utils::Uuid,
};
use eve_rs::AsError;
use k8s_openapi::{
	api::{
		apps::v1::{Deployment, DeploymentSpec},
		core::v1::{
			Container,
			ContainerPort,
			EnvVar,
			LocalObjectReference,
			Pod,
			PodSpec,
			PodTemplateSpec,
			ResourceRequirements,
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
		api::resource::Quantity,
		apis::meta::v1::LabelSelector,
		util::intstr::IntOrString,
	},
};
use kube::{
	api::{DeleteParams, ListParams, LogParams, Patch, PatchParams},
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

use crate::{
	db,
	error,
	service::deployment::digitalocean,
	utils::{settings::Settings, Error},
	Database,
};

pub async fn update_kubernetes_deployment(
	workspace_id: &Uuid,
	deployment_id: &Uuid,
	name: &str,
	registry: &str,
	image_name: &str,
	image_tag: &str,
	region: &Uuid,
	machine_type: &Uuid,
	deploy_on_push: bool,
	min_horizontal_scale: u16,
	max_horizontal_scale: u16,
	ports: &BTreeMap<u16, ExposedPortType>,
	environment_variables: &BTreeMap<String, EnvironmentVariableValue>,
	urls: &[EntryPointMapping],
	config: &Settings,
) -> Result<(), Error> {
	let kubernetes_client = get_kubernetes_config(config).await?;

	let request_id = Uuid::new_v4();
	log::trace!(
		"Deploying the container with id: {} on kubernetes with request_id: {}",
		deployment_id,
		request_id,
	);

	// new name for the docker image
	let new_repo_name = format!(
		"registry.digitalocean.com/{}/{}",
		config.digitalocean.registry, deployment_id,
	);

	let mut machine_type: BTreeMap<String, Quantity> = BTreeMap::new();

	// TODO get this from machine type
	let (ram, cpu) = ("1G".to_string(), "1.0".to_string());

	machine_type.insert("memory".to_string(), Quantity(ram));
	machine_type.insert("cpu".to_string(), Quantity(cpu));

	log::trace!(
		"request_id: {} - Deploying deployment: {}",
		request_id,
		deployment_id,
	);

	// TODO: change the namespace to workspace id
	let namespace = "default";

	let mut labels: BTreeMap<String, String> = BTreeMap::new();
	labels.insert("app".to_owned(), deployment_id.to_string());

	log::trace!(
		"request_id: {} - generating deployment configuration",
		request_id
	);

	let kubernetes_deployment = Deployment {
		metadata: ObjectMeta {
			name: Some(deployment_id.to_string()),
			namespace: Some(namespace.to_string()),
			labels: Some(labels.clone()),
			..ObjectMeta::default()
		},
		spec: Some(DeploymentSpec {
			replicas: Some(min_horizontal_scale as i32),
			selector: LabelSelector {
				match_expressions: None,
				match_labels: Some(labels.clone()),
			},
			template: PodTemplateSpec {
				spec: Some(PodSpec {
					containers: vec![Container {
						name: deployment_id.to_string(),
						image: Some(new_repo_name.to_string()),
						ports: Some(
							ports
								.iter()
								.map(|(port, _)| ContainerPort {
									container_port: *port as i32,
									..ContainerPort::default()
								})
								.collect::<Vec<_>>(),
						),
						env: Some(
							environment_variables
								.iter()
								.filter_map(|(name, value)| {
									use EnvironmentVariableValue::*;
									Some(EnvVar {
										name: name.to_string(),
										value: Some(match value {
											String(value) => value.to_string(),
											Secret { .. } => {
												return None;
											}
										}),
										..EnvVar::default()
									})
								})
								.collect::<Vec<_>>(),
						),
						resources: Some(ResourceRequirements {
							limits: Some(machine_type.clone()),
							requests: Some(machine_type.clone()),
						}),
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
			deployment_id.as_str(),
			&PatchParams::apply(deployment_id.as_str()),
			&Patch::Apply(kubernetes_deployment),
		)
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let kubernetes_service = Service {
		metadata: ObjectMeta {
			name: Some(format!("service-{}", deployment_id)),
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
			&format!("service-{}", deployment_id),
			&PatchParams::apply(&format!("service-{}", deployment_id)),
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
			format!("{}.patr.cloud", deployment_id),
		);

		vec![
			IngressRule {
				host: Some(format!("{}.patr.cloud", deployment_id)),
				http: Some(HTTPIngressRuleValue {
					paths: vec![HTTPIngressPath {
						backend: IngressBackend {
							service: Some(IngressServiceBackend {
								name: format!("service-{}", deployment_id),
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
								name: format!("service-{}", deployment_id),
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
			host: Some(format!("{}.patr.cloud", deployment_id)),
			http: Some(HTTPIngressRuleValue {
				paths: vec![HTTPIngressPath {
					backend: IngressBackend {
						service: Some(IngressServiceBackend {
							name: format!("service-{}", deployment_id),
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
				hosts: Some(vec![format!("{}.patr.cloud", deployment_id)]),
				secret_name: Some(format!("tls-{}", deployment_id)),
			},
			IngressTLS {
				hosts: Some(vec![domain]),
				secret_name: Some(format!("custom-tls-{}", deployment_id)),
			},
		]
	} else {
		log::trace!(
			"request_id: {} - adding patr domain config to ingress",
			request_id
		);
		vec![IngressTLS {
			hosts: Some(vec![format!("{}.patr.cloud", deployment_id)]),
			secret_name: Some(format!("tls-{}", deployment_id)),
		}]
	};

	let kubernetes_ingress: Ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(format!("ingress-{}", deployment_id)),
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
			&format!("ingress-{}", deployment_id),
			&PatchParams::apply(&format!("ingress-{}", deployment_id)),
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
		deployment_id
	);

	Ok(())
}

pub(super) async fn delete_kubernetes_deployment(
	deployment_id: &Uuid,
	config: &Settings,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - deleting the image from registry",
		request_id
	);
	let kubernetes_client = kube::Client::try_default()
		.await
		.expect("Expected a valid KUBECONFIG environment variable.");

	if !app_exists(deployment_id, kubernetes_client.clone(), "default").await? {
		log::trace!(
			"request_id: {} - App doesn't exist as {}",
			request_id,
			deployment_id
		);
		log::trace!(
			"request_id: {} - deployment deleted successfully!",
			request_id
		);
		Ok(())
	} else {
		log::trace!(
			"request_id: {} - App exists as {}",
			request_id,
			deployment_id
		);
		digitalocean::delete_image_from_digitalocean_registry(
			deployment_id,
			config,
		)
		.await?;

		log::trace!("request_id: {} - deleting the deployment", request_id);
		// TODO: add namespace to the database
		// TODO: add code for catching errors
		let _deployment_api =
			Api::<Deployment>::namespaced(kubernetes_client.clone(), "default")
				.delete(deployment_id.as_str(), &DeleteParams::default())
				.await?;
		let _service_api =
			Api::<Service>::namespaced(kubernetes_client.clone(), "default")
				.delete(
					&format!("service-{}", deployment_id),
					&DeleteParams::default(),
				)
				.await?;
		let _ingress_api =
			Api::<Ingress>::namespaced(kubernetes_client, "default")
				.delete(
					&format!("ingress-{}", deployment_id),
					&DeleteParams::default(),
				)
				.await?;
		log::trace!(
			"request_id: {} - deployment deleted successfully!",
			request_id
		);
		Ok(())
	}
}

pub(super) async fn get_container_logs(
	deployment_id: &Uuid,
	request_id: Uuid,
	config: &Settings,
) -> Result<String, Error> {
	// TODO: interact with prometheus to get the logs

	let kubernetes_client = get_kubernetes_config(config).await?;

	log::trace!(
		"request_id: {} - retreiving deployment info from db",
		request_id
	);

	// TODO: change log to stream_log when eve gets websockets
	// TODO: change customise LogParams for different types of logs
	// TODO: this is a temporary log retrieval method, use prometheus to get the
	// logs
	let pod_api = Api::<Pod>::namespaced(kubernetes_client, "default");

	let pod_name = pod_api
		.list(&ListParams {
			label_selector: Some(format!("app={}", deployment_id)),
			..ListParams::default()
		})
		.await?
		.items
		.into_iter()
		.next()
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?
		.metadata
		.name
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	let deployment_logs =
		pod_api.logs(&pod_name, &LogParams::default()).await?;

	log::trace!("request_id: {} - logs retreived successfully!", request_id);
	Ok(deployment_logs)
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

async fn app_exists(
	deployment_id: &Uuid,
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, Error> {
	let deployment_app =
		Api::<Deployment>::namespaced(kubernetes_client, namespace)
			.get(deployment_id.as_str())
			.await;

	if deployment_app.is_err() {
		// TODO: catch the not found error here
		return Ok(false);
	}

	Ok(true)
}

// TODO: add the logic of errored deployment
pub async fn get_kubernetes_deployment_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	deployment_id: &Uuid,
	config: &Settings,
) -> Result<DeploymentStatus, Error> {
	let deployment = db::get_deployment_by_id(connection, deployment_id)
		.await?
		.status(404)
		.body(error!(RESOURCE_DOES_NOT_EXIST).to_string())?;

	let kubernetes_client = get_kubernetes_config(config).await?;
	let deployment_status =
		Api::<Deployment>::namespaced(kubernetes_client.clone(), "default")
			.get(&deployment.id.as_str())
			.await?
			.status
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

	if deployment_status.available_replicas ==
		Some(deployment.min_horizontal_scale.into())
	{
		Ok(DeploymentStatus::Running)
	} else if deployment_status.available_replicas <=
		Some(deployment.min_horizontal_scale.into())
	{
		Ok(DeploymentStatus::Deploying)
	} else {
		Ok(DeploymentStatus::Errored)
	}
}
