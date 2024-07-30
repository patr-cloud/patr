use std::error::Error;

use models::api::workspace::domain::*;

use crate::prelude::*;

#[server(ListDomains, endpoint = "/domain-config/domain/list")]
pub async fn list_domains(
	access_token: Option<String>,
	workspace_id: Option<String>,
) -> Result<GetDomainsForWorkspaceResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use constants::USER_AGENT_STRING;

	let _ = access_token.ok_or(ServerFnError::WrappedServerError(
		ErrorType::MalformedAccessToken,
	));
	let _ = workspace_id.ok_or(ServerFnError::WrappedServerError(
		ErrorType::WrongParameters,
	));

	let domain_id = Uuid::from_str("67e5504410b1426f9247bb680e5fe0c8")
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let domain = WithId::<WorkspaceDomain>::new(
		domain_id,
		WorkspaceDomain {
			domain: Domain {
				name: "some".to_string(),
				last_unverified: None,
			},
			is_verified: true,
			nameserver_type: DomainNameserverType::Internal,
		},
	);
	let api_response = Ok(GetDomainsForWorkspaceResponse {
		domains: vec![domain],
	});

	api_response.map_err(|_: Box<dyn Error>| {
		ServerFnError::WrappedServerError(ErrorType::InternalServerError)
	})
}
