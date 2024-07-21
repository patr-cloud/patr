use std::collections::BTreeMap;

use models::api::workspace::deployment::*;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
struct Env {
	key: String,
	value: String,
}

#[server(CreateDeploymentFn, endpoint = "/infrastructure/deployment/create")]
pub async fn create_deployment(
	workspace_id: Option<String>,
	access_token: Option<String>,
	name: String,
	registry_name: String,
	image_name: String,
	image_tag: String,
	runner: String,
	machine_type: String,
	#[server(default)] env: Vec<Env>,
	port: u16,
	port_protocol: String,
	min_horizontal_scale: u16,
	max_horizontal_scale: u16,
) -> Result<CreateDeploymentResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;
	logging::log!(
		"\n\n======================{:?} {:?} {:?}--------------------\n\n",
		env,
		port,
		port_protocol
	);

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let machine_type = Uuid::parse_str(machine_type.as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let runner = Uuid::parse_str(runner.as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let registry = if registry_name.contains("patr") {
		DeploymentRegistry::PatrRegistry {
			registry: PatrRegistry,
			repository_id: Uuid::parse_str(image_name.as_str())
				.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?,
		}
	} else {
		DeploymentRegistry::ExternalRegistry {
			registry: registry_name,
			image_name,
		}
	};

	let port = StringifiedU16::new(port);
	// let port_protocol = ExposedPortType::from_str(port_protocol.as_str())
	// 	.map_err(|_|
	// ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let running_details = DeploymentRunningDetails {
		deploy_on_push: false,
		min_horizontal_scale,
		max_horizontal_scale,
		environment_variables: BTreeMap::from([]),
		ports: BTreeMap::from([]),
		volumes: BTreeMap::from([]),
		startup_probe: None,
		liveness_probe: None,
		config_mounts: BTreeMap::from([]),
	};

	let api_response = make_api_call::<CreateDeploymentRequest>(
		ApiRequest::builder()
			.path(CreateDeploymentPath { workspace_id })
			.query(())
			.headers(CreateDeploymentRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(CreateDeploymentRequest {
				name,
				image_tag,
				registry,
				machine_type,
				runner,
				deploy_on_create: false,
				running_details,
			})
			.build(),
	)
	.await;

	if api_response.is_ok() {
		leptos_axum::redirect("/deployment");
	}

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
