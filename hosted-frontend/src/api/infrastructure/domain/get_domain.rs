use models::api::workspace::{domain::*, managed_url::*};

use crate::prelude::*;

#[server(GetDomainFn, endpoint = "/domain-config/domain/get")]
pub async fn get_domain(
	access_token: Option<String>,
	workspace_id: Option<String>,
	domain_id: Option<String>,
) -> Result<GetDomainInfoInWorkspaceResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let domain_id = Uuid::parse_str(domain_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let api_response = GetDomainInfoInWorkspaceResponse {
		workspace_domain: WithId {
			id: domain_id,
			data: WorkspaceDomain {
				domain: Domain {
					name: "some".to_string(),
					last_unverified: None,
				},
				is_verified: true,
				nameserver_type: DomainNameserverType::Internal,
			},
		},
	};

	Ok(api_response)
}
