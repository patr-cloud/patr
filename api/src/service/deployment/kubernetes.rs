use std::{
	collections::BTreeMap,
	ops::DerefMut,
	process::Stdio,
	str,
	time::Duration,
};

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
use tokio::{process::Command, time};
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
	let kubernetes_client = kube::Client::try_default()
		.await
		.expect("Expected a valid KUBECONFIG environment variable.");
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
	log::trace!("request_id: {} - Pushing to {}", request_id, new_repo_name);

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

	let deployment_api =
		if app_exists(&deployment_id, kubernetes_client.clone(), &namespace)
			.await?
		{
			log::trace!(
				"request_id: {} - App exists as {:?}",
				request_id,
				deployment_id_string
			);
			// the function to create a new deployment
			log::trace!("request_id: {} - Redeploying app", request_id);
			redeploy_application(
				&deployment_id_string,
				kubernetes_client.clone(),
				&namespace,
				labels.clone(),
				deployment.horizontal_scale as i32,
				&new_repo_name,
				&request_id,
			)
			.await?
		} else {
			// if the app doesn't exists then create a new app
			create_app(
				&deployment_id_string,
				&namespace,
				labels.clone(),
				kubernetes_client.clone(),
				deployment.horizontal_scale as i32,
				&new_repo_name,
				&request_id,
			)
			.await?
		};

	// wait for the app to be completed to be deployed
	// TODO: Wait for the app to be ready
	log::trace!("request_id: {} - Waiting for app to be ready", request_id);
	wait_for_app_deploy(
		&deployment_id_string,
		deployment_api,
		deployment.horizontal_scale as i32,
		&request_id,
	)
	.await?;
	log::trace!(
		"request_id: {} - App ingress is at {}.patr.cloud",
		request_id,
		deployment_id_string
	);

	// update DNS
	log::trace!("request_id: {} - updating DNS", request_id);
	super::add_cname_record(
		&deployment_id_string,
		&config.ssh.host_name,
		&config,
		false,
	)
	.await?;
	log::trace!("request_id: {} - DNS Updated", request_id);

	// TODO: configure reverse proxy for kubernetes for custom domains
	// log::trace!("request_id: {} - adding reverse proxy", request_id);
	// super::update_nginx_with_all_domains_for_deployment(
	// 	&deployment_id_string,
	// 	&default_url,
	// 	deployment.domain_name.as_deref(),
	// 	&config,
	// 	request_id,
	// )
	// .await?;

	let _ = super::update_deployment_status(
		&deployment_id,
		&DeploymentStatus::Running,
	)
	.await;

	log::trace!(
		"request_id: {} - deleting image tagged with digitalocean registry",
		request_id
	);
	log::trace!(
		"request_id: {} - deleting image tagged with registry.digitalocean.com",
		request_id
	);
	let delete_result = super::delete_docker_image(&new_repo_name).await;
	if let Err(delete_result) = delete_result {
		log::error!(
			"request_id: {} - Failed to delete the image: {}, Error: {}",
			request_id,
			new_repo_name,
			delete_result.get_error()
		);
	}

	log::trace!("request_id: {} - deleting the pulled image", request_id);

	let delete_result = super::delete_docker_image(&image_id).await;
	if let Err(delete_result) = delete_result {
		log::error!(
			"Failed to delete the image: {}, Error: {}",
			image_id,
			delete_result.get_error()
		);
	}
	log::trace!("request_id: {} - Docker image deleted", request_id);

	Ok(())
}

async fn app_exists(
	deployment_id: &[u8],
	kubernetes_client: kube::Client,
	namespace: &str,
) -> Result<bool, Error> {
	let deployment_app =
		Api::<Deployment>::namespaced(kubernetes_client, namespace)
			.get(&hex::encode(&deployment_id))
			.await;

	if deployment_app.is_err() {
		// TODO: catch the not found error here
		return Ok(false);
	}

	Ok(true)
}

async fn redeploy_application(
	deployment_id_string: &str,
	kubernetes_client: kube::Client,
	namespace: &str,
	labels: BTreeMap<String, String>,
	horizontal_scale: i32,
	new_repo_name: &str,
	request_id: &Uuid,
) -> Result<Api<Deployment>, Error> {
	let kubernetes_deployment: Deployment = Deployment {
		metadata: ObjectMeta {
			name: Some(deployment_id_string.to_string()),
			namespace: Some(namespace.to_owned()),
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
							name: Some("http".to_string()),
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
		// .create(&PostParams::default(), &kubernetes_deployment)
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

	log::trace!("request_id: {} - deployment created", request_id);
	log::trace!("request_id: {} - App redeployed", request_id);

	Ok(deployment_api)
}

async fn create_app(
	deployment_id_string: &str,
	namespace: &str,
	labels: BTreeMap<String, String>,
	kubernetes_client: kube::Client,
	horizontal_scale: i32,
	new_repo_name: &str,
	request_id: &Uuid,
) -> Result<Api<Deployment>, Error> {
	let kubernetes_deployment: Deployment = Deployment {
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
		.create(&PostParams::default(), &kubernetes_deployment)
		.await?
		.status
		.status(500)
		.body(error!(SERVER_ERROR).to_string())?;

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
				host: Some(format!("{}.patr.cloud", deployment_id_string)),
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
	Ok(deployment_api)
}

async fn wait_for_app_deploy(
	deployment_id_string: &str,
	deployment_api: Api<Deployment>,
	horizontal_scale: i32,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!("request_id: {} - waiting for app to deploy", request_id);

	loop {
		let deployment_status = deployment_api
			.get(deployment_id_string)
			.await?
			.status
			.status(500)
			.body(error!(SERVER_ERROR).to_string())?;

		if deployment_status.available_replicas == Some(horizontal_scale) {
			break;
		}

		// TODO: maybe add some process kill functionality here

		time::sleep(Duration::from_secs(1)).await;
	}

	Ok(())
}
