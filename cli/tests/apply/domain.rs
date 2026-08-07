//! `patr apply` against domain resources.
//!
//! Domains can't be updated once created, so apply only ever adds them.

use cli::prelude::*;
use models::api::workspace::domain::*;
use wiremock::{
	Mock,
	MockServer,
	matchers::{method, path},
};

use super::*;
use crate::setup;

const CONFIG: &str = r#"
- type: Domain
  name: "example.com"
"#;

/// Fixed IDs so assertions can name them.
struct Ids {
	workspace: Uuid,
	domain: Uuid,
}

impl Ids {
	fn new() -> Self {
		Self {
			workspace: Uuid::parse_str("00000000000000000000000000000001").unwrap(),
			domain: Uuid::parse_str("00000000000000000000000000000021").unwrap(),
		}
	}
}

/// Mount `GET /workspace/{id}/domain` returning `domains`.
async fn mount_domain_list(server: &MockServer, ids: &Ids, domains: Vec<WithId<WorkspaceDomain>>) {
	let total = domains.len();

	Mock::given(method("GET"))
		.and(path(format!("/workspace/{}/domain", ids.workspace)))
		.respond_with(setup::success_list(
			ListDomainsInWorkspaceResponse { domains },
			total,
		))
		.mount(server)
		.await;
}

fn existing_domain(ids: &Ids) -> WithId<WorkspaceDomain> {
	WithId::new(
		ids.domain,
		WorkspaceDomain {
			name: "example.com".to_string(),
			last_verified: None,
			is_verified: true,
		},
	)
}

/// An unknown domain is added to the workspace.
#[tokio::test]
async fn create_when_none_matches() {
	let ids = Ids::new();
	let server = setup::reset().await;

	mount_domain_list(server, &ids, vec![]).await;
	Mock::given(method("POST"))
		.and(path(format!("/workspace/{}/domain", ids.workspace)))
		.respond_with(setup::success(AddDomainToWorkspaceResponse {
			id: OnlyId::only_id(ids.domain),
		}))
		.mount(server)
		.await;

	setup::apply(setup::state(ids.workspace), CONFIG, &[])
		.await
		.expect("apply failed");

	let body = sole_body::<AddDomainToWorkspaceRequest>(
		server,
		"POST",
		&format!("/workspace/{}/domain", ids.workspace),
	)
	.await;

	assert_eq!(body.domain, "example.com");
}

/// A domain that already exists is left alone rather than re-added.
#[tokio::test]
async fn existing_domain_is_a_no_op() {
	let ids = Ids::new();
	let server = setup::reset().await;

	mount_domain_list(server, &ids, vec![existing_domain(&ids)]).await;

	setup::apply(setup::state(ids.workspace), CONFIG, &[])
		.await
		.expect("apply failed");

	assert_no_writes(server).await;
}

/// A dry run writes nothing.
#[tokio::test]
async fn dry_run_does_not_write() {
	let ids = Ids::new();
	let server = setup::reset().await;

	mount_domain_list(server, &ids, vec![]).await;

	setup::apply(setup::state(ids.workspace), CONFIG, &["--dry-run"])
		.await
		.expect("dry run failed");

	assert_no_writes(server).await;
}
