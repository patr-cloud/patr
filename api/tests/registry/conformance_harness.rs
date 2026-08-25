//! Manual harness that boots the registry + docker-login endpoints on a real
//! TCP port so the OCI distribution-spec conformance suite can be run against
//! them.
//!
//! This is `#[ignore]`d — it never runs as part of the normal suite because it
//! blocks forever serving requests. Run it explicitly:
//!
//! ```sh
//! # infra must be up (pg/redis/minio) and PATR_TEST_* exported — see
//! # api/tests/registry/CONFORMANCE.md
//! cargo test -p api --test integration-tests -- \
//!     --ignored --nocapture registry::conformance_harness::conformance_harness
//! ```
//!
//! It seeds a user, workspace, API token, and two container repositories
//! (`OCI_NAMESPACE` and `OCI_CROSSMOUNT_NAMESPACE`), then serves a combined
//! router on `0.0.0.0:3000`:
//! - `/v2/*` → the OCI registry
//! - everything else → the API router (only `/auth/docker-login` matters here,
//!   which is the `Bearer realm=` the registry challenge points at)
//!
//! It prints the `OCI_*` environment the conformance container needs and then
//! blocks. Ctrl-C to stop.

use std::{
	collections::{BTreeMap, BTreeSet},
	net::SocketAddr,
};

use axum::{Router, body::Body, routing::any};
use http::Request;
use models::rbac::WorkspacePermission;
use tokio::net::TcpListener;
use tower::ServiceExt as _;

use crate::prelude::*;

#[tokio::test]
#[ignore = "manual: blocks forever serving the OCI conformance suite"]
async fn conformance_harness() {
	let setup = setup().await.expect("failed to set up harness");

	// Seed: a user with a workspace, an API token with SuperAdmin on it, and
	// two pre-created repos (the registry 404s on push to a repo that doesn't
	// exist, so the conformance namespaces must exist up front).
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let repo = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let crossmount = setup
		.create_test_container_repo(&user.access_token, workspace.id)
		.await;
	let token = setup
		.create_test_api_token(
			&user.access_token,
			BTreeSet::from([workspace.id]),
			BTreeMap::new(),
		)
		.await;

	// Build a combined router sharing the seeded state. Registry auth reads the
	// same DB/redis the token was written to.
	let state = setup.state().clone();
	let api_router = api::routes::api_patr_cloud::setup_routes(&state, ClientType::ApiToken).await;
	let registry_router = api::routes::registry_patr_cloud::setup_routes(&state).await;

	let app = Router::new().fallback(any(move |request: Request<Body>| {
		let api_router = api_router.clone();
		let registry_router = registry_router.clone();
		async move {
			// `/v2/*` is the OCI registry; everything else (only
			// `/auth/docker-login` matters to the conformance client — it's the
			// `Bearer realm=` the registry challenge points at) goes to the API
			// router.
			if request.uri().path().starts_with("/v2") {
				registry_router.oneshot(request).await
			} else {
				api_router.oneshot(request).await
			}
		}
	}));

	let listener = TcpListener::bind("0.0.0.0:3000")
		.await
		.expect("failed to bind 0.0.0.0:3000 — is something already on port 3000?");

	let namespace = format!("{}/{}", workspace.id, repo.name);
	let crossmount_namespace = format!("{}/{}", workspace.id, crossmount.name);

	println!("\n========================================================================");
	println!("  OCI conformance harness ready on http://localhost:3000");
	println!("  Run the conformance suite with:");
	println!();
	println!("    OCI_ROOT_URL=http://localhost:3000");
	println!("    OCI_NAMESPACE={namespace}");
	println!("    OCI_CROSSMOUNT_NAMESPACE={crossmount_namespace}");
	println!("    OCI_USERNAME=patr");
	println!("    OCI_PASSWORD={}", token.token);
	println!("========================================================================\n");

	axum::serve(
		listener,
		app.into_make_service_with_connect_info::<SocketAddr>(),
	)
	.await
	.expect("server error");
}
