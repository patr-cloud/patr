#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! This crate is the worker that runs on cloudflare before a request is sent to
//! any one of Patr's Kubernetes clusters.

use std::collections::HashMap;

use url::Host;
use worker::*;

use self::{models::IngressKVData, utils::constants};

mod models;
mod utils;

/// The main function that is called when a request is made to the worker.
#[event(fetch)]
pub async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
	let url = req.url()?;

	#[cfg(not(debug_assertions))]
	if url.scheme() != "https" {
		return Response::redirect({
			let mut url = url;
			url.set_scheme("https").map_err(|_| Error::BadEncoding)?;
			url
		});
	}

	let host = get_hostname_for_url(&url)?;

	let Some(kv_value) = env
		.kv(constants::INGRESS_KV)?
		.get(host)
		.json::<HashMap<String, IngressKVData>>()
		.await?
	else {
		return Response::error("not found", 404);
	};

	let Some((mount_point, value)) = kv_value
		.into_iter()
		.filter(|(mount_point, value)| {
			if value.is_redirect() {
				url.path() == mount_point
			} else {
				url.path().starts_with(mount_point)
			}
		})
		.reduce(|(mount_point_a, value_a), (mount_point_b, value_b)| {
			if value_a.is_redirect() {
				return (mount_point_a, value_a);
			}
			if mount_point_a.len() > mount_point_b.len() {
				(mount_point_a, value_a)
			} else {
				(mount_point_b, value_b)
			}
		})
	else {
		return Response::error("not found", 404);
	};

	let requested_path = get_stripped_path_by_mount_point(url.path(), mount_point);

	match value {
		IngressKVData::Redirect {
			to,
			permanent_redirect,
			http_only,
		} => Response::redirect_with_status(
			{
				let mut url = Url::parse(&to)?;

				url.set_scheme(if http_only { "http" } else { "https" })
					.map_err(|_| Error::BadEncoding)?;

				url
			},
			if permanent_redirect {
				constants::STATUS_CODE_PERMANENT_REDIRECT
			} else {
				constants::STATUS_CODE_TEMPORAL_REDIRECT
			},
		),
		IngressKVData::Proxy { to, http_only } => {
			Fetch::Request(Request::new_with_init(
				{
					let to_url = Url::parse(&to)?;
					let mut url = url;

					url.set_host(to_url.host_str())?;
					url.set_scheme(if http_only { "http" } else { "https" })
						.map_err(|_| Error::BadEncoding)?;

					url
				}
				.as_str(),
				&RequestInit {
					body: req.inner().body().map(Into::into),
					headers: req.headers().clone(),
					cf: CfProperties::new(),
					method: req.method(),
					redirect: RequestRedirect::Manual,
				},
			)?)
			.send()
			.await
		}
		IngressKVData::StaticSite {
			static_site_id,
			upload_id,
		} => {
			// Static sites only allow GET and HEAD requests
			if !matches!(req.method(), Method::Get | Method::Head) {
				return Response::error("method not allowed", 405);
			}

			let cache_store = Cache::default();

			let cache_key = format!(
				"{}: {}/{}/{}",
				req.method(),
				static_site_id,
				upload_id,
				requested_path
			);

			let cached_object = cache_store.get(&cache_key, true).await?;

			if let Some(response) = cached_object {
				return Ok(response);
			}

			let bucket = env.bucket(constants::STATIC_SITE_BUCKET)?;

			for file_to_try in [
				format!("{}/{}/{}", static_site_id, upload_id, requested_path),
				format!("{}/{}/{}.html", static_site_id, upload_id, requested_path),
				format!("{}/{}/{}.htm", static_site_id, upload_id, requested_path),
				format!("{}/{}/{}.shtml", static_site_id, upload_id, requested_path),
				format!(
					"{}/{}/{}/index.html",
					static_site_id, upload_id, requested_path
				),
				format!(
					"{}/{}/{}/index.htm",
					static_site_id, upload_id, requested_path
				),
				format!("{}/{}/404.html", static_site_id, upload_id),
				format!("{}/{}/index.html", static_site_id, upload_id),
				format!("{}/{}/index.htm", static_site_id, upload_id),
			] {
				let Some(file) = bucket.get(file_to_try).execute().await? else {
					continue;
				};

				let file_extension = requested_path
					.rsplit_once('.')
					.map(|(_, ext)| ext)
					.unwrap_or_default();

				if let Some(stripped) = url.path().strip_suffix("/index.html") {
					// /contacts/index.html will be redirected to /contacts/
					let mut response = Response::redirect({
						let new_path = format!("{}/", stripped);
						let mut url = url;

						url.set_path(&new_path);

						url
					})?;

					let cached_response = response.cloned()?;
					ctx.wait_until(async move {
						let _ = cache_store.put(cache_key, cached_response).await;
					});

					return Ok(response);
				}

				if let "html" | "htm" | "shtml" = file_extension {
					// /contacts.html will be redirected to /contacts
					let mut response = Response::redirect({
						let mut url = url;

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
					let mut headers = Headers::new();

					headers.set("etag", file.etag().as_str())?;
					headers.set("content-length", file.size().to_string().as_str())?;
					headers.set(
						"content-type",
						match file_extension {
							"html" => "text/html",
							"htm" => "text/html",
							"shtml" => "text/html",
							"xhtml" => "application/xhtml+xml",
							"css" => "text/css",
							"xml" => "text/xml",
							"atom" => "application/atom+xml",
							"rss" => "application/rss+xml",
							"js" => "application/javascript",
							"mml" => "text/mathml",
							"png" => "image/png",
							"jpg" => "image/jpeg",
							"jpeg" => "image/jpeg",
							"gif" => "image/gif",
							"ico" => "image/x-icon",
							"svg" => "image/svg+xml",
							"svgz" => "image/svg+xml",
							"tif" => "image/tiff",
							"tiff" => "image/tiff",
							"json" => "application/json",
							"pdf" => "application/pdf",
							"txt" => "text/plain",
							"mp4" => "video/mp4",
							"webm" => "video/webm",
							"mp3" => "audio/mpeg",
							"ogg" => "audio/ogg",
							"wav" => "audio/wav",
							"woff" => "application/font-woff",
							"woff2" => "application/font-woff2",
							"ttf" => "application/font-truetype",
							"otf" => "application/font-opentype",
							"eot" => "application/vnd.ms-fontobject",
							"mpg" => "video/mpeg",
							"mpeg" => "video/mpeg",
							"mov" => "video/quicktime",
							"avi" => "video/x-msvideo",
							"flv" => "video/x-flv",
							"m4v" => "video/x-m4v",
							"jad" => "text/vnd.sun.j2me.app-descriptor",
							"wml" => "text/vnd.wap.wml",
							"htc" => "text/x-component",
							"avif" => "image/avif",
							"webp" => "image/webp",
							"wbmp" => "image/vnd.wap.wbmp",
							"jng" => "image/x-jng",
							"bmp" => "image/x-ms-bmp",
							"jar" => "application/java-archive",
							"war" => "application/java-archive",
							"ear" => "application/java-archive",
							"hqx" => "application/mac-binhex40",
							"doc" => "application/msword",
							"ps" => "application/postscript",
							"eps" => "application/postscript",
							"ai" => "application/postscript",
							"rtf" => "application/rtf",
							"m3u8" => "application/vnd.apple.mpegurl",
							"kml" => "application/vnd.google-earth.kml+xml",
							"kmz" => "application/vnd.google-earth.kmz",
							"xls" => "application/vnd.ms-excel",
							"ppt" => "application/vnd.ms-powerpoint",
							"odg" => concat!("application/", "vnd.oasis.opendocument.graphics"),
							"odp" => concat!("application/vnd.oasis", ".opendocument.presentation"),
							"ods" => concat!("application/vnd.oasis", ".opendocument.spreadsheet"),
							"odt" => concat!("application/vnd.oasis", ".opendocument.text"),
							"pptx" => concat!(
								"application/vnd.openxmlformats",
								"-officedocument.presentationml.presentation"
							),
							"xlsx" => concat!(
								"application/vnd.openxmlformats",
								"-officedocument.spreadsheetml.sheet"
							),
							"docx" => concat!(
								"application/vnd.openxmlformats",
								"-officedocument.wordprocessingml.document"
							),
							"wmlc" => "application/vnd.wap.wmlc",
							"wasm" => "application/wasm",
							"7z" => "application/x-7z-compressed",
							"cco" => "application/x-cocoa",
							"jardiff" => "application/x-java-archive-diff",
							"jnlp" => "application/x-java-jnlp-file",
							"run" => "application/x-makeself",
							"pl" => "application/x-perl",
							"pm" => "application/x-perl",
							"prc" => "application/x-pilot",
							"pdb" => "application/x-pilot",
							"rar" => "application/x-rar-compressed",
							"rpm" => "application/x-redhat-package-manager",
							"sea" => "application/x-sea",
							"swf" => "application/x-shockwave-flash",
							"sit" => "application/x-stuffit",
							"tcl" => "application/x-tcl",
							"tk" => "application/x-tcl",
							"der" => "application/x-x509-ca-cert",
							"pem" => "application/x-x509-ca-cert",
							"crt" => "application/x-x509-ca-cert",
							"xpi" => "application/x-xpinstall",
							"xspf" => "application/xspf+xml",
							"zip" => "application/zip",
							"bin" => "application/octet-stream",
							"exe" => "application/octet-stream",
							"dll" => "application/octet-stream",
							"deb" => "application/octet-stream",
							"dmg" => "application/octet-stream",
							"iso" => "application/octet-stream",
							"img" => "application/octet-stream",
							"msi" => "application/octet-stream",
							"msp" => "application/octet-stream",
							"msm" => "application/octet-stream",
							"mid" => "audio/midi",
							"midi" => "audio/midi",
							"kar" => "audio/midi",
							"m4a" => "audio/x-m4a",
							"ra" => "audio/x-realaudio",
							"3gpp" => "video/3gpp",
							"3gp" => "video/3gpp",
							"ts" => "video/mp2t",
							"mng" => "video/x-mng",
							"asx" => "video/x-ms-asf",
							"asf" => "video/x-ms-asf",
							"wmv" => "video/x-ms-wmv",
							_ => "application/octet-stream",
						},
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

			Response::error("404 not found", 404)
		}
		IngressKVData::Deployment {
			deployment_id,
			port,
			region,
		} => {
			Fetch::Request(Request::new_with_init(
				url.as_str(),
				&RequestInit {
					body: req.inner().body().map(Into::into),
					headers: req.headers().clone(),
					cf: CfProperties {
						minify: Some(MinifyConfig {
							js: false,
							html: false,
							css: false,
						}),
						polish: Some(PolishConfig::Off),
						resolve_override: Some(format!(
							"https://{}-{}.{}.{}",
							port,
							deployment_id,
							region,
							constants::DEFAULT_PATR_DOMAIN
						)),
						scrape_shield: Some(true),
						..Default::default()
					},
					method: req.method(),
					redirect: RequestRedirect::Manual,
				},
			)?)
			.send()
			.await
		}
	}
}

/// Gets the path of the URL without the mount point. A request stripped of it's
/// mount point will be made in the case of static sites since they are stored
/// in a bucket with the mount point as the root.
pub fn get_stripped_path_by_mount_point(path: &str, mount_point: String) -> &str {
	path.trim_start_matches(mount_point.trim_end_matches('/'))
		.trim_start_matches('/')
		.trim_end_matches('/')
}

/// Gets the hostname of the URL. This is the domain name without the protocol
/// and port.
pub fn get_hostname_for_url(url: &Url) -> Result<&str> {
	url.host()
		.and_then(|host| match host {
			Host::Domain(host) => Some(host),
			_ => None,
		})
		.ok_or_else(|| Error::BadEncoding)
}

#[cfg(test)]
mod tests {

	mod getting_hostname {
		use url::Url;

		use crate::get_hostname_for_url;

		#[test]
		pub fn test_basic_url_with_http_protocol() {
			assert_eq!(
				get_hostname_for_url(&Url::parse("http://www.example.com").unwrap()).unwrap(),
				"www.example.com"
			);
		}

		#[test]
		pub fn test_basic_url_with_https_protocol() {
			assert_eq!(
				get_hostname_for_url(&Url::parse("https://www.example.com").unwrap()).unwrap(),
				"www.example.com"
			);
		}

		#[test]
		pub fn test_url_with_a_specific_port() {
			assert_eq!(
				get_hostname_for_url(&Url::parse("http://www.example.com:8080").unwrap()).unwrap(),
				"www.example.com"
			);
		}

		#[test]
		#[should_panic]
		pub fn test_url_without_protocol() {
			assert_eq!(
				get_hostname_for_url(&Url::parse("www.example.com").unwrap()).unwrap(),
				""
			);
		}

		#[test]
		pub fn test_url_with_path_and_query_parameters() {
			assert_eq!(
				get_hostname_for_url(
					&Url::parse("https://www.example.com/path/to/page?param1=value1&param2=value2")
						.unwrap()
				)
				.unwrap(),
				"www.example.com"
			);
		}

		#[test]
		#[should_panic]
		pub fn test_url_with_an_ip_address() {
			assert_eq!(
				get_hostname_for_url(&Url::parse("http://192.168.1.1").unwrap()).unwrap(),
				""
			);
		}

		#[test]
		pub fn test_url_with_non_standard_protocol() {
			assert_eq!(
				get_hostname_for_url(&Url::parse("customproto://www.example.com").unwrap())
					.unwrap(),
				"www.example.com"
			);
		}

		#[test]
		pub fn test_url_with_subdomains() {
			assert_eq!(
				get_hostname_for_url(&Url::parse("http://subdomain.example.com").unwrap()).unwrap(),
				"subdomain.example.com"
			);
		}

		#[test]
		pub fn test_url_with_username_and_password() {
			assert_eq!(
				get_hostname_for_url(&Url::parse("http://username:password@example.com").unwrap())
					.unwrap(),
				"example.com"
			);
		}

		#[test]
		pub fn test_url_with_special_characters() {
			assert_eq!(
				get_hostname_for_url(
					&Url::parse("http://www.example.com/path?param=va lue").unwrap()
				)
				.unwrap(),
				"www.example.com"
			);
		}

		#[test]
		pub fn test_url_with_international() {
			assert_eq!(
				get_hostname_for_url(&Url::parse("http://www.пример.рф").unwrap()).unwrap(),
				"www.xn--e1afmkfd.xn--p1ai"
			);
		}

		#[test]
		pub fn test_url_with_invalid_format() {
			assert_eq!(
				get_hostname_for_url(&Url::parse("htp://www.example.com").unwrap()).unwrap(),
				"www.example.com"
			);
		}

		#[test]
		#[should_panic]
		pub fn test_url_with_only_domain() {
			assert_eq!(
				get_hostname_for_url(&Url::parse("example").unwrap()).unwrap(),
				""
			);
		}

		#[test]
		#[should_panic]
		pub fn test_empty_url() {
			assert_eq!(get_hostname_for_url(&Url::parse("").unwrap()).unwrap(), "");
		}

		#[test]
		pub fn test_url_with_trailing_spaces() {
			assert_eq!(
				get_hostname_for_url(&Url::parse(" http://www.example.com ").unwrap()).unwrap(),
				"www.example.com"
			);
		}
	}

	mod mounting {
		use crate::get_stripped_path_by_mount_point;

		#[test]
		pub fn test_basic_url_with_matching_mount_point() {
			assert_eq!(
				get_stripped_path_by_mount_point("/hello/world", "/hello".to_string()),
				"world"
			);
		}

		#[test]
		pub fn test_url_with_trailing_slash_in_the_mount_point() {
			assert_eq!(
				get_stripped_path_by_mount_point("/hello/world", "/hello/".to_string()),
				"world"
			);
		}

		#[test]
		pub fn test_url_with_no_match_in_the_mount_point() {
			assert_eq!(
				get_stripped_path_by_mount_point("/hello/world", "/test".to_string()),
				"hello/world"
			);
		}

		#[test]
		pub fn test_url_with_the_same_mount_point() {
			assert_eq!(
				get_stripped_path_by_mount_point("/hello", "/hello".to_string()),
				""
			);
		}

		#[test]
		pub fn test_url_with_the_root_mount_point() {
			assert_eq!(
				get_stripped_path_by_mount_point("/hello/world", "/".to_string()),
				"hello/world"
			);
		}

		#[test]
		pub fn test_url_with_multiple_segments_in_the_mount_point() {
			assert_eq!(
				get_stripped_path_by_mount_point("/hello/world/2023", "/hello/world".to_string()),
				"2023"
			);
		}

		#[test]
		pub fn test_url_with_a_mount_point_that_partially_matches() {
			assert_eq!(
				get_stripped_path_by_mount_point("/hello/world", "/hello/wrong".to_string()),
				"hello/world"
			);
		}

		#[test]
		pub fn test_url_with_an_empty_mount_point() {
			assert_eq!(
				get_stripped_path_by_mount_point("/hello/world", "".to_string()),
				"hello/world"
			);
		}

		#[test]
		pub fn test_url_with_an_empty_url_and_mount_point() {
			assert_eq!(get_stripped_path_by_mount_point("", "".to_string()), "");
		}

		#[test]
		pub fn test_url_with_special_characters() {
			assert_eq!(
				get_stripped_path_by_mount_point(
					"/special%20chars/test",
					"/special%20chars".to_string()
				),
				"test"
			);
		}

		#[test]
		pub fn test_url_with_international_characters() {
			assert_eq!(
				get_stripped_path_by_mount_point("/hello/世界", "/hello".to_string()),
				"世界"
			);
		}

		#[test]
		pub fn test_url_with_trailing_spaces_in_the_mount_point() {
			assert_eq!(
				get_stripped_path_by_mount_point("/hello/world", "/hello/ ".to_string()),
				"hello/world"
			);
		}

		#[test]
		pub fn test_url_with_trailing_spaces_in_the_url() {
			assert_eq!(
				get_stripped_path_by_mount_point("/hello/world ", "/hello".to_string()),
				"world "
			);
		}

		#[test]
		pub fn test_url_with_a_long_mount_point() {
			assert_eq!(
				get_stripped_path_by_mount_point(
					"/this/is/a/long/mount/point",
					"/this/is/a/long".to_string()
				),
				"mount/point"
			);
		}

		#[test]
		pub fn test_url_with_a_mount_point_containing_special_characters() {
			assert_eq!(
				get_stripped_path_by_mount_point("/hello/world", "/hel*o".to_string()),
				"hello/world"
			);
		}
	}
}
