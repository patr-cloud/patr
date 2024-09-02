use std::collections::BTreeMap;

use models::api::workspace::deployment::*;

use crate::prelude::*;

#[server(UpdateDeploymentFn, endpoint = "/infrastructure/deployment/update")]
pub async fn update_deployment(
	workspace_id: Option<String>,
	access_token: Option<String>,
	deployment_id: Option<String>,
	name: Option<String>,
	machine_type: Option<String>,
	deploy_on_push: Option<bool>,
	min_horizontal_scale: Option<u16>,
	max_horizontal_scale: Option<u16>,
	ports: Option<BTreeMap<StringifiedU16, ExposedPortType>>,
	environment_variables: Option<BTreeMap<String, EnvironmentVariableValue>>,
	startup_probe: Option<DeploymentProbe>,
	liveness_probe: Option<DeploymentProbe>,
	config_mounts: Option<BTreeMap<String, Base64String>>,
	volumes: Option<BTreeMap<Uuid, String>>,
) -> Result<UpdateDeploymentResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;
	use models::api::workspace::deployment::*;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let deployment_id = Uuid::parse_str(deployment_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	if let Some(ref mt) = machine_type {
		let _ = Uuid::parse_str(mt.as_str())
			.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;
	}
	let machine_type = machine_type.and_then(|mt| {
		Uuid::parse_str(mt.as_str())
			.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))
			.ok()
	});

	// let volumes: Option<BTreeMap<String, DeploymentVolume>> = volumes.map(|vols|
	// { 	vols.iter()
	// 		.map(|(id, vol_info)| (id.to_string(), vol_info.to_owned()))
	// 		.collect()
	// });

	let update_deployment = UpdateDeploymentRequest {
		name,
		ports,
		machine_type,
		deploy_on_push,
		min_horizontal_scale,
		max_horizontal_scale,
		environment_variables,
		liveness_probe,
		startup_probe,
		config_mounts,
		runner: None,
		volumes: None,
	};

	let api_response = make_api_call::<UpdateDeploymentRequest>(
		ApiRequest::builder()
			.path(UpdateDeploymentPath {
				workspace_id,
				deployment_id,
			})
			.query(())
			.headers(UpdateDeploymentRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(update_deployment)
			.build(),
	)
	.await;

	api_response
		.map(|res| res.body)
		.map_err(|err| ServerFnError::WrappedServerError(err))
}
