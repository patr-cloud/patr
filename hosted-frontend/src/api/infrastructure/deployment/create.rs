use std::collections::BTreeMap;

use models::api::workspace::deployment::*;

use crate::prelude::*;

#[server(CreateDeploymentFn, endpoint = "")]
pub async fn create_deployment(
	workspace_id: Option<String>,
	access_token: Option<String>,
	name: String,
	registry_name: String,
	image_name: String,
	image_tag: String,
	deploy_on_create: bool,
	deploy_on_push: bool,
	min_horizontal_scale: u16,
	max_horizontal_scale: u16,
	runner: String,
	startup_probe: Option<(u16, String)>,
	liveness_probe: Option<(u16, String)>,
	machine_type: String,
	environment_variables: Vec<(String, EnvironmentVariableValue)>,
	volumes: Vec<(Uuid, DeploymentVolume)>,
	ports: Vec<(StringifiedU16, ExposedPortType)>,
) -> Result<CreateDeploymentResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

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

	let running_details = DeploymentRunningDetails {
		deploy_on_push,
		min_horizontal_scale,
		max_horizontal_scale,
		environment_variables: environment_variables.iter().map(|x| x.to_owned()).collect(),
		ports: ports.iter().map(|x| x.to_owned()).collect(),
		volumes: volumes.iter().map(|x| x.to_owned()).collect(),
		startup_probe: startup_probe.map(|(port, path)| DeploymentProbe { port, path }),
		liveness_probe: liveness_probe.map(|(port, path)| DeploymentProbe { port, path }),
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
				deploy_on_create,
				running_details,
			})
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
