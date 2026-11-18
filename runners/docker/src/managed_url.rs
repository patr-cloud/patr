//! Managed URLs → Caddy routes for the Docker runner.
//!
//! Each managed URL owns one config holding its route. The ingress mounts the
//! routes of every managed URL in the local state that exist.

use std::sync::LazyLock;

use models::utils::constants::MANAGED_URL_PATH_REGEX;
use regex::Regex;

use crate::{
	prelude::*,
	runner::{DockerObjectId, DockerSpec},
};

/// A host that's safe to write into a Caddyfile site address: lowercase DNS
/// labels separated by dots.
static HOST: LazyLock<Regex> = LazyLock::new(|| {
	Regex::new(r"^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)*$")
		.expect("the host regex is valid")
});

/// A path that's safe to write into a Caddyfile site address, the same rule
/// the API enforces when the URL is created.
static PATH: LazyLock<Regex> =
	LazyLock::new(|| Regex::new(MANAGED_URL_PATH_REGEX).expect("the path regex is valid"));

/// The objects a managed URL expects: its route, while the deployment it
/// points at is on this runner.
pub(crate) fn expected(
	runner: &DockerRunner,
	managed_url_id: Uuid,
	local: &LocalState,
) -> Result<ExpectedGraph<DockerObjectId, DockerSpec>, RunnerError> {
	let mut graph = ExpectedGraph::new(Owner::ManagedUrl(managed_url_id));
	let Some(managed_url) = local.managed_urls.get(&managed_url_id) else {
		return Ok(graph);
	};
	let Some((deployment, _)) = local.deployments.get(&managed_url.deployment_id) else {
		return Ok(graph);
	};

	let (id, data) = routes(runner, managed_url_id, managed_url, deployment.should_run)?;
	graph.add(id, DockerSpec::Config { data }, []);
	Ok(graph)
}

/// The config holding a managed URL's route. Its host and path are written
/// into a Caddyfile site address, so a host or path that could break out of
/// it is refused rather than rendered. A stopped deployment's route answers
/// 503.
///
/// `is_private` corresponds to runners fronted by a Cloudflare Tunnel — TLS
/// terminates at Cloudflare and `cloudflared` forwards to Caddy as HTTP. The
/// site address gets an explicit `http://` prefix so Caddy doesn't auto-enable
/// HTTPS and generate an HTTP→HTTPS redirect that would loop back through
/// `cloudflared`. Public runners terminate TLS at Caddy itself, so we leave
/// the scheme blank and let auto-HTTPS do its thing.
pub(crate) fn routes(
	runner: &DockerRunner,
	managed_url_id: Uuid,
	managed_url: &LocalManagedUrl,
	should_run: bool,
) -> Result<(DockerObjectId, Vec<u8>), RunnerError> {
	if !HOST.is_match(&managed_url.host) || !PATH.is_match(&managed_url.path) {
		return Err(RunnerError::host(format!(
			"managed URL `{managed_url_id}` has a host or path that can't be routed safely"
		)));
	}

	// The template appends `*` to the path to make Caddy's site-address path
	// matcher a prefix match instead of the default exact match — so a
	// managed URL at `/api` routes `/api`, `/api/foo`, `/api/v1/users`, etc.
	// Root-path managed URLs land on `host/*` which matches everything.
	let data = format!(
		include_str!("../../../assets/runner/Caddyfile.managed-url.template"),
		scheme = if runner.settings.data.runner_exposure_type.is_private() {
			"http://"
		} else {
			""
		},
		host = managed_url.host,
		path = managed_url.path,
		handler = if should_run {
			format!(
				"reverse_proxy {}.onpatr.local:{}",
				managed_url.deployment_id, managed_url.port
			)
		} else {
			String::from("error 503")
		},
	);

	Ok((
		DockerObjectId::Config(utils::config_name(
			&format!("patr-url-{managed_url_id}"),
			&data,
		)),
		data.into_bytes(),
	))
}
