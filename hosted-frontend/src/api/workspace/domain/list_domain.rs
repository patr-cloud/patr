use models::api::workspace::domain::*;

use crate::prelude::*;

#[server(ListDomains, endpoint = "/domain-config/domain/list")]
pub async fn list_domains(
	access_token: Option<String>,
	workspace_id: Option<Uuid>,
) -> Result<GetDomainsForWorkspaceResponse, ServerFnError<ErrorType>> {
	use std::str::FromStr;

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
	Ok(GetDomainsForWorkspaceResponse {
		domains: vec![domain],
	})
	.map_err(ServerFnError::WrappedServerError)
}
