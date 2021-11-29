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
			Pod,
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
			IngressTLS,
		},
	},
	apimachinery::pkg::{
		apis::meta::v1::LabelSelector,
		util::intstr::IntOrString,
	},
};
use kube::{
	api::{
		DeleteParams,
		ListParams,
		LogParams,
		ObjectMeta,
		Patch,
		PatchParams,
		PostParams,
	},
	Api,
};
use reqwest::Client;
use tokio::{process::Command, time};
use uuid::Uuid;

use crate::{
	db,
	error,
	models::db_mapping::DeploymentStatus,
	service::{
		self,
		deployment::digitalocean::{
			delete_image_from_digitalocean_registry,
			get_registry_auth_token,
		},
	},
	utils::{settings::Settings, Error},
};

pub(super) async fn deploy_container(
	image_id: String,
	_region: String,
	deployment_id: Vec<u8>,
	config: Settings,
) -> Result<(), Error> {
	// TODO: add namespace to the database

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
	log::trace!("request_id: {} - adding reverse proxy", request_id);
	update_ingress_with_all_domains_for_deployment(
		&deployment_id_string,
		kubernetes_client.clone(),
		deployment.domain_name.as_deref(),
		&namespace,
		&request_id,
	)
	.await?;

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

pub(super) async fn delete_deployment(
	deployment_id: &[u8],
	config: &Settings,
	request_id: Uuid,
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
			hex::encode(deployment_id)
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
			hex::encode(deployment_id)
		);
		delete_image_from_digitalocean_registry(deployment_id, config).await?;

		log::trace!("request_id: {} - deleting the deployment", request_id);
		// TODO: add namespace to the database
		// TODO: add code for catching errors
		let _deployment_api =
			Api::<Deployment>::namespaced(kubernetes_client.clone(), "default")
				.delete(&hex::encode(deployment_id), &DeleteParams::default())
				.await?;
		let _service_api =
			Api::<Service>::namespaced(kubernetes_client.clone(), "default")
				.delete(
					&format!("service-{}", &hex::encode(deployment_id)),
					&DeleteParams::default(),
				)
				.await?;
		let _ingress_api =
			Api::<Ingress>::namespaced(kubernetes_client, "default")
				.delete(
					&format!("ingress-{}", &hex::encode(deployment_id)),
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
	deployment_id: &[u8],
	request_id: Uuid,
) -> Result<String, Error> {
	// TODO: interact with prometheus to get the logs

	let kubernetes_client = kube::Client::try_default()
		.await
		.expect("Expected a valid KUBECONFIG environment variable.");

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
			label_selector: Some(format!("app={}", hex::encode(deployment_id))),
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
	annotations.insert(
		"kubernetes.io/ingress.class".to_string(),
		"nginx".to_string(),
	);

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
							service_name: Some(format!(
								"service-{}",
								&deployment_id_string
							)),
							service_port: Some(IntOrString::Int(80)),
							..IngressBackend::default()
						},
						..HTTPIngressPath::default()
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

async fn update_ingress_with_all_domains_for_deployment(
	deployment_id_string: &str,
	kubernetes_client: kube::Client,
	domain_name: Option<&str>,
	namespace: &str,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - updating ingress with all domains",
		request_id
	);
	// TODO: refactor this

	// TODO: test if the certificate exists
	// if yes then make https
	// else  {
	// 	continue with http
	// 	create certificate
	// 	make https
	// }
	// for custom domain
	// check for certificate
	// if it exists then
	// 	create https
	// else
	// 	continue with http only

	// let certificate_api = Api::<CertificateSigningRequest>::namespaced(
	// 	kubernetes_client.clone(),
	// 	namespace,
	// );

	// let certificate_check = certificate_api
	// 	.get(&format!("certificate-{}", deployment_id_string))
	// 	.await;

	update_nginx_config_for_domain_with_http_only(
		deployment_id_string,
		kubernetes_client.clone(),
		domain_name,
		namespace,
		request_id,
	)
	.await?;

	create_https_certificates_for_domain(
		&deployment_id_string,
		kubernetes_client.clone(),
		domain_name,
		namespace,
		request_id,
	)
	.await?;

	Ok(())
}

async fn update_nginx_config_for_domain_with_http_only(
	deployment_id_string: &str,
	kubernetes_client: kube::Client,
	domain_name: Option<&str>,
	namespace: &str,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - updating nginx config for domain",
		request_id
	);

	let mut annotations: BTreeMap<String, String> = BTreeMap::new();
	annotations.insert(
		"kubernetes.io/ingress.class".to_string(),
		"nginx".to_string(),
	);

	let custom_domain_rule = if let Some(domain) = domain_name {
		log::trace!("request_id: {} - custom domain present, adding domain details to the ingress", request_id);
		annotations.insert(
			"nginx.ingress.kubernetes.io/proxy-redirect-from".to_string(),
			domain.to_string(),
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
							service_name: Some(format!(
								"service-{}",
								&deployment_id_string
							)),
							service_port: Some(IntOrString::Int(80)),
							..IngressBackend::default()
						},
						..HTTPIngressPath::default()
					}],
				}),
			},
			IngressRule {
				host: domain_name.map(|d| d.to_string()),
				http: Some(HTTPIngressRuleValue {
					paths: vec![HTTPIngressPath {
						backend: IngressBackend {
							service_name: Some(format!(
								"service-{}",
								deployment_id_string
							)),
							service_port: Some(IntOrString::Int(80)),
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
						service_name: Some(format!(
							"service-{}",
							&deployment_id_string
						)),
						service_port: Some(IntOrString::Int(80)),
						..IngressBackend::default()
					},
					..HTTPIngressPath::default()
				}],
			}),
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

	log::trace!("request_id: {} - ingress updated", request_id);

	Ok(())
}

async fn create_https_certificates_for_domain(
	deployment_id_string: &str,
	kubernetes_client: kube::Client,
	domain_name: Option<&str>,
	namespace: &str,
	request_id: &Uuid,
) -> Result<(), Error> {
	log::trace!(
		"request_id: {} - creating https certificates for domain",
		request_id
	);

	let mut annotations: BTreeMap<String, String> = BTreeMap::new();
	annotations.insert(
		"kubernetes.io/ingress.class".to_string(),
		"nginx".to_string(),
	);
	annotations.insert(
		"cert-manager.io/issuer".to_string(),
		"letsencrypt-prod".to_string(),
	);

	let custom_domain_rule = if let Some(domain) = domain_name {
		annotations.insert(
			"nginx.ingress.kubernetes.io/proxy-redirect-from".to_string(),
			domain.to_string(),
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
							service_name: Some(format!(
								"service-{}",
								&deployment_id_string
							)),
							service_port: Some(IntOrString::Int(80)),
							..IngressBackend::default()
						},
						..HTTPIngressPath::default()
					}],
				}),
			},
			IngressRule {
				host: domain_name.map(|d| d.to_string()),
				http: Some(HTTPIngressRuleValue {
					paths: vec![HTTPIngressPath {
						backend: IngressBackend {
							service_name: Some(format!(
								"service-{}",
								deployment_id_string
							)),
							service_port: Some(IntOrString::Int(80)),
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
						service_name: Some(format!(
							"service-{}",
							&deployment_id_string
						)),
						service_port: Some(IntOrString::Int(80)),
						..IngressBackend::default()
					},
					..HTTPIngressPath::default()
				}],
			}),
		}]
	};

	let custom_domain_tls = if let Some(domain) = domain_name {
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

	Ok(())
}
