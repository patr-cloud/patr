use std::{collections::BTreeMap, ops::DerefMut, process::Stdio, str};

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
		networking::v1beta1::{
			HTTPIngressPath,
			HTTPIngressRuleValue,
			Ingress,
			IngressBackend,
			IngressRule,
			IngressSpec,
		},
	},
	apimachinery::pkg::{
		apis::meta::v1::LabelSelector,
		util::intstr::IntOrString,
	},
};
use kube::{
	api::{ObjectMeta, Patch, PatchParams, PostParams},
	Api,
};
use reqwest::Client;
use tokio::process::Command;
use uuid::Uuid;

use crate::{
	db,
	error,
	models::db_mapping::DeploymentStatus,
	service::{self, deployment::digitalocean::get_registry_auth_token},
	utils::{settings::Settings, Error},
};

pub(super) async fn deploy_container(
	image_id: String,
	_region: String,
	deployment_id: Vec<u8>,
	config: Settings,
) -> Result<(), Error> {
	let request_id = Uuid::new_v4();
	log::trace!("Deploying the container with id: {} and image: {} on DigitalOcean Managed Kubernetes with request_id: {}",
		hex::encode(&deployment_id),
		image_id,
		request_id
	);
	let deployment = db::get_deployment_by_id(
		service::get_app().database.acquire().await?.deref_mut(),
		&deployment_id,
	)
	.await?
	.status(500)
	.body(error!(SERVER_ERROR).to_string())?;

	let client = Client::new();
	let deployment_id_string = hex::encode(&deployment_id);

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

	log::trace!("request_id: {} - Pulling image from registry", request_id);
	super::pull_image_from_registry(&image_id, &config).await?;
	log::trace!("request_id: {} - Image pulled", request_id);

	// new name for the docker image
	let new_repo_name = format!(
		"registry.digitalocean.com/{}/{}",
		config.digitalocean.registry, deployment_id_string,
	);
	log::trace!("request_id: {} - Pushing to {}", new_repo_name, request_id);

	// rename the docker image with the digital ocean registry url
	super::tag_docker_image(&image_id, &new_repo_name).await?;
	log::trace!("request_id: {} - Image tagged", request_id);

	// Get login details from digital ocean registry and decode from base 64 to
	// binary
	let auth_token =
		base64::decode(get_registry_auth_token(&config, &client).await?)?;
	log::trace!("request_id: {} - Got auth token", request_id);

	// Convert auth token from binary to utf8
	let auth_token = str::from_utf8(&auth_token)?;
	log::trace!(
		"request_id: {} - Decoded auth token as {}",
		auth_token,
		request_id
	);

	// get username and password from the auth token
	let (username, password) = auth_token
		.split_once(":")
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	// Login into the registry
	let output = Command::new("docker")
		.arg("login")
		.arg("-u")
		.arg(username)
		.arg("-p")
		.arg(password)
		.arg("registry.digitalocean.com")
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?
		.wait()
		.await?;
	log::trace!("request_id: {} - Logged into DO registry", request_id);

	if !output.success() {
		return Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()));
	}
	log::trace!("request_id: {} - Login was success", request_id);

	// if the loggin in is successful the push the docker image to registry
	let push_status = Command::new("docker")
		.arg("push")
		.arg(&new_repo_name)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?
		.wait()
		.await?;
	log::trace!(
		"request_id: {} - Pushing to DO to {}",
		request_id,
		new_repo_name,
	);

	if !push_status.success() {
		return Err(Error::empty()
			.status(500)
			.body(error!(SERVER_ERROR).to_string()));
	}

	log::trace!("request_id: {} - Pushed to DO", request_id);
	let replicas = 1;
	let client = kube::Client::try_default()
		.await
		.expect("Expected a valid KUBECONFIG environment variable.");
	let namespace = "default";

	let mut labels: BTreeMap<String, String> = BTreeMap::new();
	labels.insert("app".to_owned(), deployment_id_string.clone());

	// Definition of the deployment. Alternatively, a YAML representation could
	// be used as well.
	log::trace!(
		"request_id: {} generating deployment configuration",
		request_id
	);
	let kubernetes_deployment: Deployment = Deployment {
		metadata: ObjectMeta {
			name: Some(deployment_id_string.clone()),
			namespace: Some(namespace.to_owned()),
			labels: Some(labels.clone()),
			..ObjectMeta::default()
		},
		spec: Some(DeploymentSpec {
			replicas: Some(replicas),
			selector: LabelSelector {
				match_expressions: None,
				match_labels: Some(labels.clone()),
			},
			template: PodTemplateSpec {
				spec: Some(PodSpec {
					containers: vec![Container {
						name: deployment_id_string.clone(),
						image: Some(new_repo_name),
						ports: Some(vec![ContainerPort {
							container_port: 80,
							host_port: Some(80),
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
	log::trace!("request_id: {} creating deployment", request_id);
	let deployment_api: Api<Deployment> =
		Api::namespaced(client.clone(), namespace);

	let params = PatchParams::apply(&deployment_id_string);

	let patch = Patch::Apply(kubernetes_deployment);

	let api = deployment_api
		.patch(&deployment_id_string, &params, &patch)
		// .create(&PostParams::default(), &kubernetes_deployment)
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	println!("DEPLOYMENT: {:#?}", api);

	let kubernetes_service: Service = Service {
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

	log::trace!("request_id: {} creating Load balancer Service", request_id);
	let service_api: Api<Service> = Api::namespaced(client.clone(), namespace);

	let params =
		PatchParams::apply(&format!("service-{}", &deployment_id_string));

	let patch = Patch::Apply(kubernetes_service);

	let api = service_api
		.patch(
			&format!("service-{}", &deployment_id_string),
			&params,
			&patch,
		)
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	println!("SERVICE: {:#?}", api);

	let mut annotations: BTreeMap<String, String> = BTreeMap::new();
	annotations
		.insert("kubernetes.io/ingress.class".to_owned(), "nginx".to_owned());

	let kubernetes_ingress: Ingress = Ingress {
		metadata: ObjectMeta {
			name: Some(format!("ingress-{}", &deployment_id_string)),
			annotations: Some(annotations),
			..ObjectMeta::default()
		},
		spec: Some(IngressSpec {
			rules: Some(vec![IngressRule {
				host: Some("test.samyak.tk".to_string()),
				http: Some(HTTPIngressRuleValue {
					paths: vec![HTTPIngressPath {
						backend: IngressBackend {
							service_name: format!(
								"service-{}",
								&deployment_id_string
							),
							service_port: IntOrString::Int(80),
						},
						path: None,
					}],
				}),
			}]),
			..IngressSpec::default()
		}),
		..Ingress::default()
	};

	let ingress_api: Api<Ingress> = Api::namespaced(client, namespace);

	let params =
		PatchParams::apply(&format!("ingress-{}", &deployment_id_string));

	let patch = Patch::Apply(kubernetes_ingress);

	let api = ingress_api
		.patch(
			&format!("ingress-{}", &deployment_id_string),
			&params,
			&patch,
		)
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	println!("INGRESS: {:#?}", api);

	log::trace!("request_id: {} deployment created", request_id);

	Ok(())
}
