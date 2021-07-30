use eve_rs::{App as EveApp, AsError, Context, NextHandler};
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::{
	core::v1::{Pod, Service},
	networking::v1beta1::Ingress,
};
use kube::{
	api::{ListParams, PostParams, WatchEvent},
	Api,
	ResourceExt,
};

use crate::{
	app::{create_eve_app, App},
	db,
	error,
	models::db_mapping::EventData,
	pin_fn,
	utils::{Error, ErrorData, EveContext, EveMiddleware},
};

/// # Description
/// This function is used to create a sub app for every endpoint listed. It
/// creates an eve app which binds the endpoint with functions.
///
/// # Arguments
/// * `app` - an object of type [`App`] which contains all the configuration of
///   api including the
/// database connections.
///
/// # Returns
/// this function returns `EveApp<EveContext, EveMiddleware, App, ErrorData>`
/// containing context, middleware, object of [`App`] and Error
///
/// [`App`]: App
pub fn create_sub_app(
	app: &App,
) -> EveApp<EveContext, EveMiddleware, App, ErrorData> {
	let mut sub_app = create_eve_app(app);

	sub_app.post(
		"/docker-registry/notification",
		[EveMiddleware::CustomFunction(pin_fn!(notification_handler))],
	);
	sub_app
}

/// # Description
/// This function is used to handle all the notifications of the API.
/// This function will detect a push being made to a tag, and in case a
/// deployment exists with the given tag, it will automatically update the
/// `deployed_image` of the given [`Deployment`] in the database
///
/// # Arguments
/// * `context` - an object of [`EveContext`] containing the request, response,
///   database connection, body,
/// state and other things
/// * ` _` -  an object of type [`NextHandler`] which is used to call the
///   function
///
/// # Returns
/// this function returns a `Result<EveContext, Error>` containing an object of
/// [`EveContext`] or an error
///
/// [`EveContext`]: EveContext
/// [`NextHandler`]: NextHandler
/// [`Deployment`]: Deployment
pub async fn notification_handler(
	mut context: EveContext,
	_: NextHandler<EveContext, ErrorData>,
) -> Result<EveContext, Error> {
	if context.get_content_type().as_str() !=
		"application/vnd.docker.distribution.events.v1+json"
	{
		Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string())?;
	}
	let body = context.get_body()?;
	let events: EventData = serde_json::from_str(&body)?;

	// check if the event is a push event
	// get image name, repository name, tag if present
	for event in events.events {
		if event.action != "push" {
			continue;
		}
		let target = event.target;
		if target.tag.is_empty() {
			continue;
		}

		let repository = target.repository;
		let mut splitter = repository.split('/');
		let org_name = if let Some(val) = splitter.next() {
			val
		} else {
			continue;
		};
		let image_name = if let Some(val) = splitter.next() {
			val
		} else {
			continue;
		};
		let tag = target.tag;

		let organisation = db::get_organisation_by_name(
			context.get_database_connection(),
			org_name,
		)
		.await?;
		if organisation.is_none() {
			continue;
		}
		let organisation = organisation.unwrap();

		let deployments =
			db::get_deployments_by_image_name_and_tag_for_organisation(
				context.get_database_connection(),
				image_name,
				&tag,
				&organisation.id,
			)
			.await?;

		for deployment in deployments {
			let full_image_name = format!(
				"{}@{}",
				deployment
					.get_full_image(context.get_database_connection())
					.await?,
				target.digest
			);

			db::update_deployment_deployed_image(
				context.get_database_connection(),
				&deployment.id,
				&full_image_name,
			)
			.await?;

			// deploy the image here controller
			let kubernetes_client = kube::Client::try_default()
				.await
				.expect("Expected a valid KUBECONFIG environment variable.");

			// Preparation of resources used by the `kube_runtime::Controller`
			let deployment_pods: Api<Pod> =
				Api::namespaced(kubernetes_client.clone(), "default");

			// prepare pod json for kubernetes
			let pod = serde_json::from_value(serde_json::json!({
				"apiVersion": "v1",
				"kind": "Pod",
				"metadata": {
					"name": &full_image_name,
					"labels": {
    					"app.kubernetes.io/component": "webserver"
					}
				},
				"spec": {
					"selector": {
    					"matchLabels": {
      						"app.kubernetes.io/component": "webserver"
						},
					},
					"containers": [
						{
							"name": format!("deployment-{}", hex::encode(&deployment.id)),
							"image": &full_image_name,
						},
					],
					"imagePullSecrets": {
						"name": "regcred"
					},
					"ports": {
						"name": "http",
						  "containerPort": "80",
						"readinessProbe": {
							"httpGet": {
								"path": "/",
								"port": "80"
							},
							"initialDelaySeconds": 10,
							  "periodSeconds": 10
						},
						"livenessProbe": {
							  "httpGet": {
								"path": "/",
								"port": "80"
							},
							"initialDelaySeconds": "10",
							  "periodSeconds": "10"
						}
					}
				}
			}))?;

			// Deployment service for exposing the app to the cluster and maintaining a contant ip address
			let deployment_service: Api<Service> =
				Api::namespaced(kubernetes_client.clone(), "default");

			let pod_service = serde_json::from_value(serde_json::json!({
				"apiVersion": "v1",
				"kind": "Service",
				"metadata" : {
					"name": format!("deployment-{}-service", hex::encode(&deployment.id)),
				},
				"labels": {
					"app.kubernetes.io/component": "webserver"
				},
				"spec": {
					"ports": {
						"name": "http",
						"port": "80",
						"targetPort": "http"
					},
					"type": "ClusterIP",
					"selector": {
						"app.kubernetes.io/component": "webserver"
					}
				}
			}))?;

			// Ingress for exposing the app to the internet
			let deployment_ingress: Api<Ingress> =
				Api::namespaced(kubernetes_client.clone(), "default");

			let pod_ingress = serde_json::from_value(serde_json::json!({
				"apiVersion": "networking.k8s.io/v1beta1",
				"kind": "Ingress",
				"metadata": {
					"annotations": {
						"cert-manager.io/cluster-issuer": "letsencrypt-ingress-prod",
						"kubernetes.io/ingress.class": "nginx",
						"nginx.ingress.kubernetes.io/force-ssl-redirect": "true"
					},
					"name": format!("deployment-{}", hex::encode(&deployment.id))
				},
				"spec": {
					"rules": {
						"host": format!("{}.vicara.tech", hex::encode(&deployment.id)),
						"http": {
							"paths": {
								"backend": {
									"serviceName": format!("deployment-{}-service", hex::encode(&deployment.id)),
									"servicePort": "80"
								}
							}
						}
					},
					"tls": {
						"hosts": format!("{}.vicara.tech", hex::encode(&deployment.id)),
						"secretName": "vicara-secret"
					}
				}
			}))?;

			let _pod =
				deployment_pods.create(&PostParams::default(), &pod).await?;

			let _service = deployment_service
				.create(&PostParams::default(), &pod_service)
				.await?;

			let _ingress = deployment_ingress
				.create(&PostParams::default(), &pod_ingress)
				.await?;
		}
	}

	Ok(context)
}
