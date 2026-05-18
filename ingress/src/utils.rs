use crate::prelude::*;

/// The base URL for fetching error pages. The `serve_error_page` function will
/// fetch error pages from this URL and serve them to the user when an error
/// occurs. The error pages are expected to be in the format
/// `{ERROR_PAGE_BASE}/{page}.html`, where `{page}` is the name of the error
/// page (e.g., "not-found", "deployment-stopped", etc.).
const ERROR_PAGE_BASE: &str = "https://assets.patr.cloud/error-pages";

/// Clones the request's headers and augments them with `X-Forwarded-Proto`,
/// `X-Forwarded-Host`, and `X-Forwarded-For` so the upstream container knows
/// the original scheme, hostname, and client IP. Workers always receive HTTPS
/// at Cloudflare's edge, so pass `"https"` unless the destination is an
/// HTTP-only managed URL.
pub fn build_forwarded_headers(req: &Request, scheme: &str) -> Result<Headers> {
	let headers = req.headers().clone();
	headers.set("X-Forwarded-Proto", scheme)?;

	let url = req.url()?;
	if let Some(host) = url.host_str() {
		headers.set("X-Forwarded-Host", host)?;
	}

	if let Ok(Some(client_ip)) = headers.get("CF-Connecting-IP") {
		headers.set("X-Forwarded-For", &client_ip)?;
	}

	Ok(headers)
}

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
