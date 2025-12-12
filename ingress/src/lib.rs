//! This crate is the worker that runs on cloudflare before a request is sent to
//! any one of Patr's Kubernetes clusters.
use url::Host;
use worker::*;

use self::utils::constants;

/// Handling of any route that comes into a Patr domain.
mod internal;
/// Handling of routes that comes as a Managed URL.
mod managed_url;
/// All the utility functions and structs.
mod utils;

/// Prelude module to re-export commonly used items.
pub mod prelude {
	pub use models::cloudflare::kv::*;
	pub use worker::*;

	pub use crate::utils::constants;
}

/// The main function that is called when a request is made to the worker.
#[event(fetch)]
pub async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
	let url = req.url()?;

	// Redirect all http requests to https
	#[cfg(not(debug_assertions))]
	if url.scheme() != "https" {
		return Response::redirect({
			let mut url = url;
			url.set_scheme("https").map_err(|_| Error::BadEncoding)?;
			url
		});
	}

	let host = url
		.host()
		.and_then(|host| match host {
			Host::Domain(host) => Some(host),
			_ => None,
		})
		.ok_or_else(|| Error::BadEncoding)?;

	if host.ends_with(constants::DEFAULT_PATR_DOMAIN) {
		internal::handle_request(req, env, ctx, host).await
	} else {
		managed_url::handle_request(req, env, ctx, host).await
	}
}
