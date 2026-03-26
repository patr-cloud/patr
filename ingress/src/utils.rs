use crate::prelude::*;

const ERROR_PAGE_BASE: &str = "https://assets.patr.cloud/error-pages";

/// Fetches a branded error page from assets.patr.cloud and returns it as the
/// response with the given status code. The user's URL stays unchanged in the
/// browser. Falls back to a plain text error if the fetch fails.
pub async fn serve_error_page(page: &str, status: u16) -> Result<Response> {
	let url = format!("{ERROR_PAGE_BASE}/{page}.html");

	let fetched = Fetch::Url(Url::parse(&url).map_err(|_| Error::BadEncoding)?)
		.send()
		.await;

	match fetched {
		Ok(mut resp) => {
			let body = resp.bytes().await?;
			let mut response = Response::from_bytes(body)?;
			response = response.with_status(status);
			let headers = response.headers_mut();
			headers.set("Content-Type", "text/html; charset=utf-8")?;
			headers.set("Server", "patr")?;
			Ok(response)
		}
		Err(_) => Response::error(page, status),
	}
}

/// Constants used in the Worker
pub mod constants {
	/// The default domain for the PATR platform. Any requests to this domain
	/// will be either a deployment or a static site that has the default domain
	pub const DEFAULT_PATR_DOMAIN: &str = "onpatr.cloud";

	/// The cloudflare KV namespace that stores the ingress configuration
	pub const INGRESS_KV: &str = "INGRESS_KV";
	/// The cloudflare R2 bucket that stores all the static sites
	pub const STATIC_SITE_BUCKET: &str = "STATIC_SITE_BUCKET";

	/// The default status code for a temporary redirect
	pub const STATUS_CODE_TEMPORAL_REDIRECT: u16 = 307;
	/// The default status code for a permanent redirect
	pub const STATUS_CODE_PERMANENT_REDIRECT: u16 = 308;
}
