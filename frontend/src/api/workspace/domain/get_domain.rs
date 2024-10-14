use models::api::workspace::domain::*;

use crate::prelude::*;

#[server(GetDomainFn, endpoint = "/domain-config/domain/get")]
pub async fn get_domain(
	access_token: Option<String>,
	workspace_id: Option<Uuid>,
	domain_id: Option<String>,
) -> Result<GetDomainInfoInWorkspaceResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;
	let _access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let _workspace_id = workspace_id
		.ok_or_else(|| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

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
