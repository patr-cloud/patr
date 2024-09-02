use std::str::FromStr;

use convert_case::{Case, *};
use models::api::workspace::managed_url::*;

use crate::prelude::*;

pub fn get_managed_url_type(
	url_type: String,
	url: String,
	port: u16,
	http_only: bool,
	permanent_redirect: bool,
) -> Option<ManagedUrlType> {
	match ManagedUrlTypeDiscriminant::from_str(url_type.to_case(Case::Camel).as_str()) {
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
	workspace_id: Option<Uuid>,
	access_token: Option<String>,
	sub_domain: String,
	domain_id: String,
	path: String,
	url_type: String,
	url: String,
	port: u16,
	http_only: bool,
	permanent_redirect: bool,
) -> Result<CreateManagedURLResponse, ServerFnError<ErrorType>> {
	let access_token = BearerToken::from_str(access_token.unwrap().as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::MalformedAccessToken))?;

	let workspace_id = Uuid::parse_str(&workspace_id.unwrap().to_string())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let domain_id = Uuid::parse_str(domain_id.as_str())
		.map_err(|_| ServerFnError::WrappedServerError(ErrorType::WrongParameters))?;

	let url_type = get_managed_url_type(url_type, url, port, http_only, permanent_redirect).ok_or(
		ServerFnError::WrappedServerError(ErrorType::WrongParameters),
	)?;

	let req_body = CreateManagedURLRequest {
		sub_domain,
		domain_id,
		path,
		url_type,
	};

	make_api_call::<CreateManagedURLRequest>(
		ApiRequest::builder()
			.path(CreateManagedURLPath { workspace_id })
			.query(())
			.headers(CreateManagedURLRequestHeaders {
				authorization: access_token,
				user_agent: UserAgent::from_static("todo"),
			})
			.body(req_body)
			.build(),
	)
	.await
	.map(|res| {
		leptos_axum::redirect("/managed-url");
		res.body
	})
	.map_err(ServerFnError::WrappedServerError)
}
