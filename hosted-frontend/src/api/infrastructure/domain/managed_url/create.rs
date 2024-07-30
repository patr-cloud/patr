use std::str::FromStr;

use models::api::workspace::managed_url::*;

use crate::prelude::*;

pub fn get_managed_url_type(
	url_type: String,
	url: String,
	port: u16,
	http_only: bool,
	permanent_redirect: bool,
) -> Option<ManagedUrlType> {
	match ManagedUrlTypeDiscriminant::from_str(url_type.as_str()) {
		Ok(ManagedUrlTypeDiscriminant::ProxyDeployment) => {
			let deployment_id = Uuid::from_str(url.as_str()).ok()?;
			Some(ManagedUrlType::ProxyDeployment {
				deployment_id,
				port,
			})
		}
		Ok(ManagedUrlTypeDiscriminant::ProxyStaticSite) => {
			let static_site_id = Uuid::from_str(url.as_str()).ok()?;
			Some(ManagedUrlType::ProxyStaticSite { static_site_id })
		}
		Ok(ManagedUrlTypeDiscriminant::ProxyUrl) => {
			Some(ManagedUrlType::ProxyUrl { url, http_only })
		}
		Ok(ManagedUrlTypeDiscriminant::Redirect) => Some(ManagedUrlType::Redirect {
			url,
			permanent_redirect,
			http_only,
		}),
		_ => None,
	}
}

#[server(CreateManagedURLs, endpoint = "/domain-config/managed-url/create")]
pub async fn create_managed_url(
	workspace_id: Option<String>,
	access_token: Option<String>,
	sub_domain: String,
	domain_id: String,
	path: String,
	// url_type: ManagedUrlType,
	url_type: String,
	url: String,
	port: u16,
	http_only: bool,
	permanent_redirect: bool,
) -> Result<CreateManagedURLResponse, ServerFnError<ErrorType>> {
	use constants::USER_AGENT_STRING;

	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(workspace_id.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let domain_id = Uuid::parse_str(domain_id.as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let manage_url_type = get_managed_url_type(url_type, url, port, http_only, permanent_redirect)
		.ok_or(ServerFnError::WrappedServerError(
			ErrorType::WrongParameters,
		))?;

	let req_body = CreateManagedURLRequest {
		sub_domain,
		domain_id,
		path,
		url_type: ManagedUrlType::ProxyUrl {
			url: "vicara.co".to_string(),
			http_only: false,
		},
	};

	let api_response: Result<_, _> = make_api_call::<CreateManagedURLRequest>(
		ApiRequest::builder()
			.path(CreateManagedURLPath { workspace_id })
			.query(())
			.headers(CreateManagedURLRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static(USER_AGENT_STRING),
			})
			.body(req_body)
			.build(),
	)
	.await;

	if api_response.is_ok() {
		leptos_axum::redirect("/managed-url")
	}

	api_response
		.map(|res| res.body)
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::InternalServerError))
}
