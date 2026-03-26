use models::prelude::Uuid;

use crate::{prelude::*, utils::serve_error_page};

/// Handles a ProxyStaticSite managed URL. Serves files from R2 with caching,
/// path resolution fallbacks, and HTML extension stripping redirects.
pub async fn handle_static_site(
	req: &Request,
	url: &Url,
	env: &Env,
	ctx: &Context,
	requested_path: &str,
	static_site_id: Uuid,
	upload_id: Uuid,
) -> Result<Response> {
	// Static sites only allow GET and HEAD requests
	if !matches!(req.method(), Method::Get | Method::Head) {
		return Response::error("method not allowed", 405);
	}

	let cache_store = Cache::default();
	let cache_key = format!("{}/{}/{}", static_site_id, upload_id, requested_path);

	if let Some(response) = cache_store.get(&cache_key, true).await? {
		return Ok(response);
	}

	let bucket = env.bucket(constants::STATIC_SITE_BUCKET)?;

	for file_to_try in [
		format!("{static_site_id}/{upload_id}/{requested_path}"),
		format!("{static_site_id}/{upload_id}/{requested_path}.html"),
		format!("{static_site_id}/{upload_id}/{requested_path}.htm"),
		format!("{static_site_id}/{upload_id}/{requested_path}.shtml"),
		format!("{static_site_id}/{upload_id}/{requested_path}/index.html"),
		format!("{static_site_id}/{upload_id}/{requested_path}/index.htm"),
		format!("{static_site_id}/{upload_id}/404.html"),
		format!("{static_site_id}/{upload_id}/index.html"),
		format!("{static_site_id}/{upload_id}/index.htm"),
	] {
		let Some(file) = bucket.get(&file_to_try).execute().await? else {
			continue;
		};

		let file_extension = file_to_try
			.rsplit_once('.')
			.map(|(_, ext)| ext)
			.unwrap_or_default();

		if let Some(stripped) = url.path().strip_suffix("/index.html") {
			// /contacts/index.html will be redirected to /contacts/
			let mut response = Response::redirect({
				let new_path = format!("{stripped}/");
				let mut url = url.clone();
				url.set_path(&new_path);
				url
			})?;

			let cached_response = response.cloned()?;
			let cache_key = cache_key.clone();
			ctx.wait_until(async move {
				let _ = cache_store.put(cache_key, cached_response).await;
			});

			return Ok(response);
		}

		if let "html" | "htm" | "shtml" = file_extension {
			// /contacts.html will be redirected to /contacts
			let mut response = Response::redirect({
				let mut url = url.clone();
				let new_path = url
					.path()
					.trim_end_matches(".html")
					.trim_end_matches(".htm")
					.trim_end_matches(".shtml")
					.to_string();
				url.set_path(&new_path);
				url
			})?;

			let cached_response = response.cloned()?;
			let cache_key = cache_key.clone();
			ctx.wait_until(async move {
				let _ = cache_store.put(cache_key, cached_response).await;
			});

			return Ok(response);
		}

		let mut response = {
			if req.method() == Method::Head {
				Response::empty()
			} else {
				Response::from_stream(file.body().unwrap().stream()?)
			}
		}?
		.with_headers({
			let headers = Headers::new();

			headers.set("etag", file.etag().as_str())?;
			headers.set("content-length", file.size().to_string().as_str())?;
			headers.set(
				"content-type",
				mime_guess::from_ext(file_extension)
					.first_or_octet_stream()
					.as_ref(),
			)?;
			headers.set("last-modified", file.uploaded().to_string().as_str())?;

			headers
		})
		.with_status(200);

		let cached_response = response.cloned()?;
		ctx.wait_until(async move {
			let _ = cache_store.put(cache_key, cached_response).await;
		});

		return Ok(response);
	}

	serve_error_page("not-found", 404).await
}
