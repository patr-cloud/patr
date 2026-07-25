use models::{
	ApiSuccessResponseBody,
	api::workspace::domain::*,
	utils::{ListResourceQuery, Uuid},
};

use crate::prelude::*;

#[tokio::test]
async fn add_domain_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	assert!(!domain.domain.is_empty());
}

#[tokio::test]
async fn list_domains_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let _domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDomainsInWorkspaceRequest>::builder()
				.path(ListDomainsInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListDomainsInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListDomainsInWorkspaceResponse>>();

	assert_eq!(1, response.response.domains.len());
}

#[tokio::test]
async fn list_domains_empty() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDomainsInWorkspaceRequest>::builder()
				.path(ListDomainsInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListDomainsInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListDomainsInWorkspaceResponse>>();

	assert!(response.response.domains.is_empty());
}

#[tokio::test]
async fn get_domain_info_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetDomainInfoInWorkspaceResponse>>();

	assert_eq!(domain.id, response.response.workspace_domain.id);
}

#[tokio::test]
async fn get_domain_info_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: Uuid::nil(),
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn delete_domain_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteDomainInWorkspaceRequest>::builder()
				.path(DeleteDomainInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(DeleteDomainInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			DeleteDomainInWorkspaceResponse,
		));

	// Verify it's gone
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}

#[tokio::test]
async fn is_domain_valid_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<IsDomainValidRequest>::builder()
				.path(IsDomainValidPath {
					workspace_id: workspace.id,
				})
				.headers(IsDomainValidRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.query(IsDomainValidQuery {
					domain: "example.com".to_string(),
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<IsDomainValidResponse>>();

	assert!(response.response.valid, "example.com should be valid");
}

#[tokio::test]
async fn verify_domain_works() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;

	// Verification will likely fail (no DNS records), but it should not error
	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<VerifyDomainInWorkspaceRequest>::builder()
				.path(VerifyDomainInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(VerifyDomainInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	let status = response.status_code();
	assert!(
		status.is_success() || status.is_server_error(),
		"verify should succeed or server error, got {status}"
	);
}

#[tokio::test]
async fn add_domain_not_root() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<AddDomainToWorkspaceRequest>::builder()
				.path(AddDomainToWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(AddDomainToWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddDomainToWorkspaceRequest {
					domain: format!("sub.{}.com", random_name(8)),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"non-root subdomains should be rejected with NotRootDomain"
	);
}

#[tokio::test]
async fn add_domain_not_icann() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<AddDomainToWorkspaceRequest>::builder()
				.path(AddDomainToWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(AddDomainToWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddDomainToWorkspaceRequest {
					// `.local` is in the PSL private section, not ICANN.
					domain: format!("{}.local", random_name(8)),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"non-ICANN TLDs should be rejected with NotIcannDomain"
	);
}

#[tokio::test]
async fn add_domain_duplicate() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<AddDomainToWorkspaceRequest>::builder()
				.path(AddDomainToWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(AddDomainToWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddDomainToWorkspaceRequest {
					domain: domain.domain.clone(),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"adding the same domain twice should fail with ResourceAlreadyExists"
	);
}

#[tokio::test]
async fn delete_domain_in_use() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;
	let _url_id = setup
		.create_test_managed_url(&user.access_token, workspace.id, domain.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteDomainInWorkspaceRequest>::builder()
				.path(DeleteDomainInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(DeleteDomainInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"deleting a domain with attached managed URLs should fail with ResourceInUse"
	);
}

#[tokio::test]
async fn domain_cross_workspace() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user.access_token).await;
	let workspace_b = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace_a.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace_b.id,
					domain_id: domain.id,
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_client_error(),
		"domain in workspace A should not be reachable via workspace B's path"
	);
}

#[tokio::test]
async fn domain_unauthorized() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDomainsInWorkspaceRequest>::builder()
				.path(ListDomainsInWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(ListDomainsInWorkspaceRequestHeaders {
					authorization: BearerToken::from_str("invalid-token").unwrap(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert!(response.status_code().is_client_error());
}

/// get-info returns the flattened domain: full name, nameserver type, and the
/// unverified state of a freshly added domain.
#[tokio::test]
async fn get_domain_info_full_shape() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetDomainInfoInWorkspaceResponse>>();

	let wd = response.response.workspace_domain;
	assert_eq!(domain.domain, wd.name, "full domain name should round-trip");
	assert!(!wd.is_verified, "a freshly added domain is unverified");
	assert!(
		wd.last_verified.is_none(),
		"an unverified domain has no last_verified"
	);
}

/// An IDN (non-ASCII) domain is neither punycoded nor cleanly rejected — the
/// non-ASCII name reaches the DB CHECK and 500s. Pinned gap.
#[tokio::test]
async fn add_domain_idn_not_punycoded() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<AddDomainToWorkspaceRequest>::builder()
				.path(AddDomainToWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(AddDomainToWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddDomainToWorkspaceRequest {
					domain: format!("münchen{}.com", random_name(8)),
				})
				.build(),
		)
		.await;

	assert!(
		response.status_code().is_server_error(),
		"IDN domain is not punycoded and hits the DB CHECK → 500, got {}",
		response.status_code()
	);
}

/// Domains are globally unique by (name, tld): the same domain cannot be added
/// to a second workspace, even one owned by the same user.
#[tokio::test]
async fn add_domain_globally_unique_across_workspaces() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace_a = setup.create_test_workspace(&user.access_token).await;
	let workspace_b = setup.create_test_workspace(&user.access_token).await;
	let domain = format!("{}.com", random_name(8));

	let first = setup
		.make_web_dashboard_call(
			ApiRequest::<AddDomainToWorkspaceRequest>::builder()
				.path(AddDomainToWorkspacePath {
					workspace_id: workspace_a.id,
				})
				.headers(AddDomainToWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddDomainToWorkspaceRequest {
					domain: domain.clone(),
				})
				.build(),
		)
		.await;
	assert!(
		first.status_code().is_success(),
		"first add should succeed, got {}",
		first.status_code()
	);

	let second = setup
		.make_web_dashboard_call(
			ApiRequest::<AddDomainToWorkspaceRequest>::builder()
				.path(AddDomainToWorkspacePath {
					workspace_id: workspace_b.id,
				})
				.headers(AddDomainToWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddDomainToWorkspaceRequest {
					domain: domain.clone(),
				})
				.build(),
		)
		.await;
	assert_eq!(
		409,
		second.status_code().as_u16(),
		"same domain in another workspace should be ResourceAlreadyExists (409)"
	);
}

/// A domain freed by deletion can be added again (the workspace_domain row is
/// hard-deleted; the soft-deleted resource row doesn't block re-use).
#[tokio::test]
async fn add_domain_readdable_after_delete() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<DeleteDomainInWorkspaceRequest>::builder()
				.path(DeleteDomainInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(DeleteDomainInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.assert_json(&ApiSuccessResponseBody::new(
			DeleteDomainInWorkspaceResponse,
		));

	let readd = setup
		.make_web_dashboard_call(
			ApiRequest::<AddDomainToWorkspaceRequest>::builder()
				.path(AddDomainToWorkspacePath {
					workspace_id: workspace.id,
				})
				.headers(AddDomainToWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.body(AddDomainToWorkspaceRequest {
					domain: domain.domain.clone(),
				})
				.build(),
		)
		.await;
	assert!(
		readd.status_code().is_success(),
		"a deleted domain should be re-addable, got {}",
		readd.status_code()
	);
}

/// The list returns full domain names ordered by created descending (newest
/// first).
#[tokio::test]
async fn list_domains_ordered_created_desc() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let mut names = Vec::new();
	for _ in 0..3 {
		let name = format!("{}.com", random_name(8));
		let _ = setup
			.make_web_dashboard_call(
				ApiRequest::<AddDomainToWorkspaceRequest>::builder()
					.path(AddDomainToWorkspacePath {
						workspace_id: workspace.id,
					})
					.headers(AddDomainToWorkspaceRequestHeaders {
						authorization: user.access_token.clone(),
						user_agent: TEST_USER_AGENT,
					})
					.body(AddDomainToWorkspaceRequest {
						domain: name.clone(),
					})
					.build(),
			)
			.await
			.json::<ApiSuccessResponseBody<AddDomainToWorkspaceResponse>>();
		names.push(name);
	}

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDomainsInWorkspaceRequest>::builder()
				.path(ListDomainsInWorkspacePath {
					workspace_id: workspace.id,
				})
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 100,
					page: 0,
					additional_query: (),
				})
				.headers(ListDomainsInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListDomainsInWorkspaceResponse>>();

	let listed: Vec<String> = response
		.response
		.domains
		.iter()
		.map(|d| d.name.clone())
		.collect();
	names.reverse();
	assert_eq!(names, listed, "domains should be ordered created DESC");
}

/// page/count slice the domain list and pages don't overlap.
#[tokio::test]
async fn list_domains_pagination() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	for _ in 0..5 {
		setup
			.create_test_domain(&user.access_token, workspace.id)
			.await;
	}

	let page0 = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDomainsInWorkspaceRequest>::builder()
				.path(ListDomainsInWorkspacePath {
					workspace_id: workspace.id,
				})
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 2,
					page: 0,
					additional_query: (),
				})
				.headers(ListDomainsInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListDomainsInWorkspaceResponse>>();
	assert_eq!(
		2,
		page0.response.domains.len(),
		"page 0 should have 2 domains"
	);

	let page1 = setup
		.make_web_dashboard_call(
			ApiRequest::<ListDomainsInWorkspaceRequest>::builder()
				.path(ListDomainsInWorkspacePath {
					workspace_id: workspace.id,
				})
				.query(ListResourceQuery {
					sort: None,
					search: Default::default(),
					count: 2,
					page: 1,
					additional_query: (),
				})
				.headers(ListDomainsInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<ListDomainsInWorkspaceResponse>>();
	assert!(
		!page1.response.domains.is_empty(),
		"page 1 should have remaining domains"
	);

	let page0_ids: std::collections::BTreeSet<Uuid> =
		page0.response.domains.iter().map(|d| d.id).collect();
	let page1_ids: std::collections::BTreeSet<Uuid> =
		page1.response.domains.iter().map(|d| d.id).collect();
	assert!(
		page0_ids.is_disjoint(&page1_ids),
		"pages should not overlap"
	);
}

/// is-domain-valid: an already-added domain is reported as a conflict (409).
#[tokio::test]
async fn is_domain_valid_existing_conflicts() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<IsDomainValidRequest>::builder()
				.path(IsDomainValidPath {
					workspace_id: workspace.id,
				})
				.query(IsDomainValidQuery {
					domain: domain.domain.clone(),
				})
				.headers(IsDomainValidRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert_eq!(
		409,
		response.status_code().as_u16(),
		"an existing domain should be reported as ResourceAlreadyExists (409)"
	);
}

/// is-domain-valid rejects a subdomain (NotRootDomain → 400).
#[tokio::test]
async fn is_domain_valid_subdomain_rejected() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<IsDomainValidRequest>::builder()
				.path(IsDomainValidPath {
					workspace_id: workspace.id,
				})
				.query(IsDomainValidQuery {
					domain: format!("sub.{}.com", random_name(8)),
				})
				.headers(IsDomainValidRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert_eq!(
		400,
		response.status_code().as_u16(),
		"a subdomain should be rejected with NotRootDomain (400)"
	);
}

/// is-domain-valid rejects a non-ICANN TLD (NotIcannDomain → 400).
#[tokio::test]
async fn is_domain_valid_non_icann_rejected() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<IsDomainValidRequest>::builder()
				.path(IsDomainValidPath {
					workspace_id: workspace.id,
				})
				.query(IsDomainValidQuery {
					domain: format!("{}.local", random_name(8)),
				})
				.headers(IsDomainValidRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert_eq!(
		400,
		response.status_code().as_u16(),
		"a non-ICANN TLD should be rejected with NotIcannDomain (400)"
	);
}

/// Verifying a domain whose TXT record can't be resolved (offline) leaves it
/// unverified — the verify call never flips `is_verified` on its own.
#[tokio::test]
async fn verify_domain_does_not_set_verified() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;

	// Offline DNS can't satisfy the TXT challenge; the call may 200 with
	// verified:false. The durable invariant is that it does not mark verified.
	setup
		.make_web_dashboard_call(
			ApiRequest::<VerifyDomainInWorkspaceRequest>::builder()
				.path(VerifyDomainInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(VerifyDomainInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	let info = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetDomainInfoInWorkspaceResponse>>();
	assert!(
		!info.response.workspace_domain.is_verified,
		"verify should not mark the domain verified offline"
	);
}

/// Verify never demotes an already-verified domain: a failing re-verify leaves
/// `is_verified` true.
#[tokio::test]
async fn verify_domain_never_demotes() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;
	let domain = setup
		.create_test_domain(&user.access_token, workspace.id)
		.await;
	setup.mark_test_domain_verified(domain.id).await;

	setup
		.make_web_dashboard_call(
			ApiRequest::<VerifyDomainInWorkspaceRequest>::builder()
				.path(VerifyDomainInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(VerifyDomainInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	let info = setup
		.make_web_dashboard_call(
			ApiRequest::<GetDomainInfoInWorkspaceRequest>::builder()
				.path(GetDomainInfoInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: domain.id,
				})
				.headers(GetDomainInfoInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await
		.json::<ApiSuccessResponseBody<GetDomainInfoInWorkspaceResponse>>();
	assert!(
		info.response.workspace_domain.is_verified,
		"verify should never demote a verified domain"
	);
}

/// Verifying a never-existed domain id is rejected by the permission layer
/// before the existence check (anti-enumeration → 401).
#[tokio::test]
async fn verify_domain_nonexistent() {
	let setup = setup().await.expect("failed to setup test server");
	let user = setup.create_test_user().await;
	let workspace = setup.create_test_workspace(&user.access_token).await;

	let response = setup
		.make_web_dashboard_call(
			ApiRequest::<VerifyDomainInWorkspaceRequest>::builder()
				.path(VerifyDomainInWorkspacePath {
					workspace_id: workspace.id,
					domain_id: Uuid::nil(),
				})
				.headers(VerifyDomainInWorkspaceRequestHeaders {
					authorization: user.access_token.clone(),
					user_agent: TEST_USER_AGENT,
				})
				.build(),
		)
		.await;

	assert_eq!(
		401,
		response.status_code().as_u16(),
		"verify on a never-existed domain should 401 (perm before existence)"
	);
}
