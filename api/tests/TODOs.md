# Unit Test TODOs

## Auth (`auth.rs`)

### Missing Edge Cases

- [ ] `create_account_username_starts_with_dot` — rejects `.foo` (must start with `[a-z0-9_]`)
- [ ] `create_account_username_ends_with_dot` — rejects `foo.` (must end with `[a-z0-9_]`)
- [ ] `create_account_username_with_uppercase` — rejects `FooBar`
- [ ] `create_account_invalid_email` — malformed email rejected
- [ ] `complete_sign_up_otp_wrong_format` — non-matching `^\d{3}-?\d{3}$`

# Integration Test TODOs

Comprehensive list of missing test cases. Organized by module.

**Current state:** ~186 integration tests + 6 unit tests across 14 test files covering ~60% of 124 declared endpoints.

---

## Auth (`auth.rs`)

### Missing Edge Cases

- [ ] `create_account_duplicate_email` — same email, different username
- [ ] `complete_sign_up_expired_otp` — OTP used after expiry window
- [ ] `complete_sign_up_already_completed` — double-join attempt
- [ ] `login_case_insensitive_username` — login with different casing
- [ ] `login_with_mfa_required` — returns `MfaRequired` error when MFA active
- [ ] `login_with_mfa_valid_otp` — full MFA login flow
- [ ] `login_with_mfa_invalid_otp` — MFA OTP wrong → `MfaOtpInvalid`
- [ ] `renew_access_token_expired` — expired refresh token rejected
- [ ] `forgot_password_nonexistent_user` — no error leak (silent success)
- [ ] `forgot_password_rate_limit` — repeated calls throttled
- [ ] `reset_password_expired_otp` — OTP past expiry
- [ ] `reset_password_new_password_invalid` — new password fails validation
- [ ] `resend_otp_nonexistent_user` — graceful handling

### OAuth Endpoints (entirely untested)

- [ ] `oauth_authorize_works` — GET `/auth/oauth/authorize`
- [ ] `oauth_authorize_invalid_client` — bad client_id
- [ ] `oauth_token_works` — POST `/auth/oauth/token`
- [ ] `oauth_token_invalid_grant` — invalid authorization code
- [ ] `oauth_introspect_works` — POST `/auth/oauth/introspect`
- [ ] `oauth_introspect_expired_token` — expired token returns inactive
- [ ] `oauth_revoke_token_works` — POST `/auth/oauth/revoke`
- [ ] `oauth_revoke_already_revoked` — idempotent revocation

### Docker Auth Edge Cases

- [ ] `docker_login_expired_token` — stale credentials
- [ ] `docker_login_invalid_format` — malformed basic auth header

---

## User (`user.rs`)

### Missing Endpoints

- [ ] `update_user_email_works` — POST `/user/update-email`
- [ ] `update_user_email_already_taken` — → `EmailUnavailable`
- [ ] `update_user_email_invalid` — malformed email
- [ ] `verify_user_email_works` — POST `/user/verify-email`
- [ ] `verify_user_email_wrong_otp` — invalid OTP
- [ ] `verify_user_email_expired_otp` — OTP past expiry
- [ ] `update_user_phone_number_works` — POST `/user/update-phone-number`
- [ ] `update_user_phone_number_invalid_country_code` — not `^[A-Z]{2}$`
- [ ] `update_user_phone_number_invalid_number` — not `^\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}$`
- [ ] `update_user_phone_number_already_taken` — → `PhoneUnavailable`
- [ ] `verify_user_phone_number_works` — POST `/user/verify-phone-number`
- [ ] `verify_user_phone_number_wrong_otp` — invalid OTP

### Web Logins (entirely untested)

- [ ] `list_web_logins_works` — GET `/user/login`
- [ ] `list_web_logins_empty` — no active sessions
- [ ] `get_web_login_info_works` — GET `/user/login/{login_id}`
- [ ] `get_web_login_info_nonexistent` — → `ResourceDoesNotExist`
- [ ] `delete_web_login_works` — DELETE `/user/login/{login_id}` (session revocation)
- [ ] `delete_web_login_current_session` — revoking own session

### Edge Cases

- [ ] `update_user_info_empty_fields` — PATCH with no changes
- [ ] `change_password_same_as_current` — old == new
- [ ] `change_password_new_invalid` — new password fails validation
- [ ] `search_for_user_partial_match` — substring matching behavior
- [ ] `search_for_user_special_chars` — SQL injection-safe search
- [ ] `get_user_details_own_id` — viewing self via user_id

---

## User MFA (`user_mfa.rs`)

### Missing Tests

- [ ] `activate_mfa_works` — full activation with valid TOTP code
- [ ] `deactivate_mfa_works` — DELETE `/user/mfa`
- [ ] `deactivate_mfa_when_inactive` — → `MfaAlreadyInactive`
- [ ] `activate_mfa_when_already_active` — → `MfaAlreadyActive`
- [ ] `activate_mfa_expired_secret` — secret generated too long ago
- [ ] `get_mfa_secret_regenerates` — calling twice gives new secret

---

## User API Token (`user_api_token.rs`)

### Missing Edge Cases

- [ ] `create_api_token_duplicate_name` — → `ApiTokenAlreadyExists`
- [ ] `api_token_with_ip_restriction` — allowed IPs enforced
- [ ] `api_token_blocked_ip` — → `DisallowedIpAddressForApiToken`
- [ ] `use_api_token_for_auth` — API token in `Authorization` header works for API calls
- [ ] `use_revoked_api_token` — revoked token rejected
- [ ] `api_token_with_permissions` — scoped permissions honored
- [ ] `api_token_without_permissions` — default permissions
- [ ] `update_api_token_name_conflict` — rename to existing name
- [ ] `list_api_tokens_pagination` — verify ordering/limits

---

## Workspace (`workspace.rs`)

### Missing Tests

- [ ] `delete_workspace_works` — currently `#[ignore]` due to audit_log FK; unblock and test
- [ ] `delete_workspace_not_empty` — workspace with resources → `WorkspaceNotEmpty`
- [ ] `delete_workspace_with_deployments` — FK constraint blocks delete
- [ ] `delete_workspace_with_volumes` — FK constraint blocks delete
- [ ] `delete_workspace_with_domains` — FK constraint blocks delete
- [ ] `create_workspace_name_too_short` — < 4 chars rejected (`RESOURCE_NAME_REGEX`)
- [ ] `create_workspace_name_too_long` — > 255 chars rejected
- [ ] `create_workspace_name_special_chars` — chars outside `[a-zA-Z0-9\-_ .]` rejected
- [ ] `update_workspace_name_conflict` — rename to taken name → `WorkspaceNameAlreadyExists`
- [ ] `update_workspace_unauthorized` — non-member cannot update
- [ ] `list_user_workspaces_multiple` — user in multiple workspaces

---

## Deployment (`workspace_deployment.rs`)

### Missing Tests

- [ ] `create_deployment_duplicate_name` — same name in workspace → `ResourceAlreadyExists`
- [ ] `create_deployment_invalid_machine_type` — nonexistent machine type
- [ ] `create_deployment_with_volumes` — attach volumes on create
- [ ] `create_deployment_with_env_vars` — environment variables
- [ ] `create_deployment_with_ports` — port configuration
- [ ] `update_deployment_name` — rename deployment
- [ ] `update_deployment_machine_type` — change machine type
- [ ] `update_deployment_image` — change container image (triggers deploy)
- [ ] `start_deployment_already_running` — idempotent or error
- [ ] `stop_deployment_already_stopped` — idempotent or error
- [ ] `delete_deployment_while_running` — must stop first or cascades
- [ ] `get_deployment_logs_empty` — no logs yet
- [ ] `get_deployment_metric_empty` — no metrics yet
- [ ] `deployment_cross_workspace` — deployment in workspace A not accessible from workspace B

### Deploy History

- [ ] `delete_deploy_history_works` — DELETE `.../deploy-history/{digest}`
- [ ] `delete_deploy_history_nonexistent` — invalid digest → `ResourceDoesNotExist`
- [ ] `revert_deployment_works` — POST `.../deploy-history/{digest}/revert`
- [ ] `revert_deployment_nonexistent_digest` — bad digest
- [ ] `revert_deployment_to_current` — revert to already-active image
- [ ] `list_deploy_history_after_multiple_deploys` — ordered correctly

---

## Runner (`workspace_runner.rs`)

### Missing Tests

- [ ] `add_runner_duplicate_name` — same name → `ResourceAlreadyExists`
- [ ] `add_runner_invalid_name` — name outside `RESOURCE_NAME_REGEX`
- [ ] `runner_already_connected` — → `RunnerAlreadyConnected`
- [ ] `runner_invalid_mode` — → `InvalidRunnerMode`
- [ ] `get_ingress_token_nonexistent_runner` — → `ResourceDoesNotExist`
- [ ] `runner_cross_workspace` — runner in workspace A not visible from B

---

## Domain (`workspace_domain.rs`)

### DNS Records (entirely untested)

- [ ] `add_dns_record_works` — POST `.../domain/{id}/dns-record`
- [ ] `add_dns_record_invalid_name` — fails `DNS_RECORD_NAME_REGEX`
- [ ] `get_domain_dns_records_works` — GET `.../domain/{id}/dns-record`
- [ ] `get_domain_dns_records_empty` — no records
- [ ] `update_dns_record_works` — PATCH `.../domain/{id}/dns-record/{record_id}`
- [ ] `update_dns_record_nonexistent` — → `ResourceDoesNotExist`
- [ ] `delete_dns_record_works` — DELETE `.../domain/{id}/dns-record/{record_id}`
- [ ] `delete_dns_record_nonexistent` — → `ResourceDoesNotExist`

### Domain Verification

- [ ] `get_verification_records_works` — GET `.../domain/{id}/verification-records`
- [ ] `get_verification_records_nonexistent_domain` — → `ResourceDoesNotExist`
- [ ] `is_domain_personal_works` — GET `/workspace/{id}/is-domain-personal`
- [ ] `verify_domain_already_verified` — double-verify behavior
- [ ] `verify_domain_unverifiable` — DNS not configured

### Edge Cases

- [ ] `add_domain_not_root` — → `NotRootDomain`
- [ ] `add_domain_not_icann` — → `NotIcannDomain`
- [ ] `add_domain_duplicate` — → `ResourceAlreadyExists`
- [ ] `delete_domain_in_use` — domain with managed URLs → `ResourceInUse`
- [ ] `domain_cross_workspace` — domain in workspace A not visible from B

---

## Managed URL (`workspace_managed_url.rs`)

### Missing URL Type Tests

- [ ] `create_managed_url_proxy_deployment` — type `ProxyDeployment` with `deployment_id` + `port`
- [ ] `create_managed_url_proxy_static_site` — type `ProxyStaticSite` with `static_site_id`
- [ ] `create_managed_url_proxy_url` — type `ProxyUrl` with `url` + `http_only`
- [ ] `create_managed_url_redirect` — type `Redirect` with `url` + `permanent_redirect` + `http_only`
- [ ] `create_managed_url_redirect_permanent` — `permanent_redirect: true`
- [ ] `create_managed_url_invalid_deployment_id` — nonexistent deployment
- [ ] `create_managed_url_invalid_static_site_id` — nonexistent static site
- [ ] `create_managed_url_unverified_domain` — domain not verified yet

### Edge Cases

- [ ] `update_managed_url_change_type` — switch from proxy to redirect
- [ ] `get_managed_url_info` — no GET single endpoint exists; verify list returns all fields
- [ ] `delete_managed_url_nonexistent` — → `ResourceDoesNotExist`
- [ ] `verify_configuration_not_configured` — misconfigured URL
- [ ] `managed_url_cross_workspace` — URL in workspace A not visible from B

---

## Container Registry (`workspace_container_registry.rs`)

### Missing Endpoint Tests

- [ ] `get_manifest_details_works` — GET `.../manifest/{digest_or_tag}`
- [ ] `get_manifest_details_nonexistent` — → `ResourceDoesNotExist`
- [ ] `get_exposed_ports_works` — GET `.../manifest/{digest_or_tag}/exposed-ports`
- [ ] `get_exposed_ports_no_ports` — image with no EXPOSE
- [ ] `delete_manifest_works` — DELETE `.../manifest/{digest_or_tag}`
- [ ] `delete_manifest_nonexistent` — → `ResourceDoesNotExist`

### Push/Pull Flow

- [ ] `push_image_and_list_tags` — push via Docker, verify tags appear
- [ ] `push_image_and_list_manifests` — push via Docker, verify manifests appear
- [ ] `push_multiple_tags` — same image, multiple tags
- [ ] `delete_tag_in_use` — tag referenced by deployment

### Edge Cases

- [ ] `create_repository_invalid_name` — name outside `RESOURCE_NAME_REGEX`
- [ ] `delete_repository_with_images` — repository not empty
- [ ] `container_registry_cross_workspace` — repo in workspace A not visible from B

---

## Volume (`workspace_volume.rs`)

### Missing Tests

- [ ] `create_volume_name_too_short` — < 4 chars
- [ ] `create_volume_name_too_long` — > 255 chars
- [ ] `update_volume_increase_size` — size increase accepted
- [ ] `update_volume_decrease_size` — → `CannotReduceVolumeSize`
- [ ] `delete_volume_attached_to_deployment` — → `ResourceInUse`
- [ ] `create_volume_exceeds_limit` — → `CannotAddNewVolume`
- [ ] `volume_cross_workspace` — volume in workspace A not visible from B

---

## Secret (entirely untested module)

- [ ] `create_secret_works` — POST `/workspace/{id}/secret`
- [ ] `create_secret_duplicate_name` — → `ResourceAlreadyExists`
- [ ] `create_secret_invalid_name` — name validation
- [ ] `list_secrets_works` — GET `/workspace/{id}/secret`
- [ ] `list_secrets_empty` — no secrets
- [ ] `update_secret_works` — PATCH `/workspace/{id}/secret/{secret_id}`
- [ ] `update_secret_nonexistent` — → `ResourceDoesNotExist`
- [ ] `delete_secret_works` — DELETE `/workspace/{id}/secret/{secret_id}`
- [ ] `delete_secret_nonexistent` — → `ResourceDoesNotExist`
- [ ] `delete_secret_in_use` — secret referenced by deployment → `ResourceInUse`
- [ ] `secret_unauthorized` — non-member cannot access
- [ ] `secret_cross_workspace` — secret in workspace A not visible from B

---

## Static Site (entirely untested module)

- [ ] `create_static_site_works` — POST `.../static-site`
- [ ] `create_static_site_invalid_name` — name validation
- [ ] `create_static_site_duplicate_name` — → `ResourceAlreadyExists`
- [ ] `list_static_sites_works` — GET `.../static-site`
- [ ] `list_static_sites_empty` — no sites
- [ ] `get_static_site_info_works` — GET `.../static-site/{id}`
- [ ] `get_static_site_info_nonexistent` — → `ResourceDoesNotExist`
- [ ] `update_static_site_works` — PATCH `.../static-site/{id}`
- [ ] `delete_static_site_works` — DELETE `.../static-site/{id}`
- [ ] `start_static_site_works` — POST `.../static-site/{id}/start`
- [ ] `stop_static_site_works` — POST `.../static-site/{id}/stop`
- [ ] `upload_static_site_works` — POST `.../static-site/{id}/upload`
- [ ] `list_upload_history_works` — GET `.../static-site/{id}/upload`
- [ ] `list_upload_history_empty` — no uploads
- [ ] `revert_static_site_works` — POST `.../static-site/{id}/revert`
- [ ] `static_site_unauthorized` — non-member cannot access
- [ ] `static_site_cross_workspace` — site in workspace A not visible from B

---

## Database (entirely untested module)

- [ ] `create_database_works` — POST `.../infrastructure/database`
- [ ] `create_database_invalid_name` — name validation
- [ ] `create_database_duplicate_name` — → `ResourceAlreadyExists`
- [ ] `list_databases_works` — GET `.../infrastructure/database`
- [ ] `list_databases_empty` — no databases
- [ ] `get_database_info_works` — GET `.../infrastructure/database/{id}`
- [ ] `get_database_info_nonexistent` — → `ResourceDoesNotExist`
- [ ] `delete_database_works` — DELETE `.../infrastructure/database/{id}`
- [ ] `delete_database_nonexistent` — → `ResourceDoesNotExist`
- [ ] `list_database_machine_types_works` — GET `/workspace/infrastructure/database/plan`
- [ ] `database_unauthorized` — non-member cannot access
- [ ] `database_cross_workspace` — database in workspace A not visible from B

---

## RBAC (`workspace_rbac.rs`)

### Missing Tests

- [ ] `create_role_duplicate_name` — → `RoleAlreadyExists`
- [ ] `create_role_invalid_name` — name outside `RESOURCE_NAME_REGEX`
- [ ] `delete_role_in_use` — role assigned to users → `RoleInUse`
- [ ] `delete_role_nonexistent` — → `RoleDoesNotExist`
- [ ] `update_role_nonexistent` — → `RoleDoesNotExist`
- [ ] `update_role_add_permissions` — add permissions to existing role
- [ ] `update_role_remove_permissions` — remove permissions from existing role
- [ ] `update_user_roles_nonexistent_user` — user not in workspace
- [ ] `update_user_roles_nonexistent_role` — → `RoleDoesNotExist`
- [ ] `remove_user_from_workspace_not_member` — user not in workspace
- [ ] `remove_self_from_workspace` — super admin removing self
- [ ] `add_user_to_workspace_already_member` — duplicate add
- [ ] `list_users_for_role_empty` — no users assigned

---

## Permissions (`permissions.rs`)

### Missing Include/Exclude Tests

- [ ] `volume_include_specific` — include list limits volume access
- [ ] `domain_include_specific` — include list limits domain access
- [ ] `container_registry_include_specific` — include list limits repo access
- [ ] `managed_url_include_specific` — include list limits managed URL access
- [ ] `runner_include_specific` — include list limits runner access
- [ ] `deployment_exclude_specific` — exclude list blocks specific deployment
- [ ] `volume_exclude_specific` — exclude list blocks specific volume
- [ ] `runner_exclude_specific` — exclude list blocks specific runner
- [ ] `domain_exclude_specific` — exclude list blocks specific domain
- [ ] `container_registry_exclude_specific` — exclude list blocks specific repo

### Missing Cross-Permission Tests

- [ ] `volume_view_doesnt_grant_delete` — view permission insufficient for delete
- [ ] `volume_view_doesnt_grant_edit` — view permission insufficient for edit
- [ ] `domain_view_doesnt_grant_delete` — view permission insufficient for delete
- [ ] `container_registry_view_doesnt_grant_delete` — view insufficient for delete
- [ ] `managed_url_view_doesnt_grant_delete` — view insufficient for delete
- [ ] `runner_view_doesnt_grant_create` — view insufficient for create

### API Token Permission Tests

- [ ] `api_token_with_workspace_permissions` — token with scoped workspace access
- [ ] `api_token_denied_without_permission` — token lacks required permission
- [ ] `api_token_resource_level_permissions` — token with include/exclude lists

---

## Service Account

### Missing Tests (require `ClientType::ApiToken` test server)

The test infra only supports `ClientType::WebDashboard` (JWT auth). Tests that authenticate
using `patrv1.*` tokens cannot be written until a `ClientType::ApiToken` test server is added.

- [ ] `service_account_token_authenticates` — create SA with Runner::View role, use SA token to call GetRunnerInfo
- [ ] `service_account_token_deleted_sa_fails` — delete SA, use its token, expect auth failure
- [ ] `sa_without_runner_permission_denied` — SA with no permissions, use SA token to access runner
- [ ] `sa_with_runner_execute_can_access_runner` — SA with Runner::Execute, use SA token for runner endpoint
- [ ] `regenerate_token_invalidates_old` — regenerate token, use old token, expect auth failure
- [ ] `user_api_token_still_works_after_sa_feature` — regression: user API token still authenticates

**Fix:** See "ClientType Refactor" section below — same blocker.

---

## ClientType Refactor — Per-Endpoint Allowed Client Types

The endpoint declaration system was changed from `api = bool` to `client_type = [...]` arrays.
Each endpoint now explicitly declares which client types can call it (`WebDashboard`, `ApiToken`,
`ServiceAccount`). The auth layer enforces this at runtime via `E::ALLOWED_CLIENT_TYPES`. Most
workspace endpoints accept all three; SA management and `create_workspace` exclude SA;
`stream_runner_data_for_workspace` is SA-only; loki/mimir push is SA-only.

This is a HUGE behavioral change and needs comprehensive coverage. Split into two phases.

### Phase 1: Test Infrastructure

The current `tests/setup.rs` only mounts one server (`ClientType::WebDashboard`). To test
the ApiToken and ServiceAccount flows, we need to mount the api.patr.cloud server too.

- [ ] **Add second `TestServer` in `setup.rs`** mounted with
      `&[ClientType::ApiToken, ClientType::ServiceAccount]` — represents the api.patr.cloud
      server. Bind to a separate port. Store on `TestSetup` as `api_token_server`.
- [ ] **Add `make_api_token_call()` method on `TestSetup`** — sends requests to the api.patr.cloud
      server using the provided `BearerToken`. Mirrors `make_api_call()` signature.
- [ ] **Add `make_sa_call()` helper** — convenience wrapper around `make_api_token_call()` that
      accepts a `TestServiceAccount` and constructs the bearer token from `sa.token`.
- [ ] **Add `create_test_user_api_token()` helper** — creates a user API token via the existing
      `CreateApiToken` endpoint, returns a `TestUserApiToken { id, token, user_id }`. Mirrors
      `create_test_service_account` pattern.
- [ ] **Add helpers for token construction** — `BearerToken::from_str(&sa.token)` and similar,
      to keep test bodies readable.

### Phase 2: Tests

Once Phase 1 is in place, write the following. Group into a new test module
`api/tests/api/client_type.rs` for the cross-cutting tests; restriction-specific tests live
in their respective module files.

#### 2a. `get_user_data_for_token` dispatch (unit-style integration tests)

Verify the prefix-based dispatch works correctly. Located in `api/tests/api/auth/`.

- [ ] `dispatch_patrv1_token_routes_to_api_token_module` — token starting with `patrv1.` is
      handled by the api_token module. Verify by sending a valid user API token to an endpoint
      that allows `[ApiToken]` and asserting success.
- [ ] `dispatch_jwt_token_routes_to_web_dashboard_module` — JWT token is handled by the
      web_dashboard module. Verify with existing web dashboard auth flow.
- [ ] `dispatch_garbage_token_returns_malformed_access_token` — random non-prefix string
      falls to JWT parsing → `MalformedAccessToken`.
- [ ] `dispatch_malformed_patrv1_token_returns_malformed_api_token` — `patrv1.notauuid.alsogarbage`
      → `MalformedApiToken`.

#### 2b. `client_type` resolution on `RequestUserData`

Verify the `client_type` field is correctly set by each auth module.

- [ ] `client_type_set_to_api_token_for_user_token` — auth with user API token →
      `user_data.client_type == ClientType::ApiToken`. Use a debug-only echo endpoint, or
      assert via observable side effects.
- [ ] `client_type_set_to_service_account_for_sa_token` — auth with SA token →
      `user_data.client_type == ClientType::ServiceAccount`.
- [ ] `client_type_set_to_web_dashboard_for_jwt` — auth with JWT → `client_type == WebDashboard`.

(Note: these may need a small test-only endpoint that echoes `user_data.client_type` in the
response. Alternatively, assert through behavioral effects in 2c below.)

#### 2c. `ALLOWED_CLIENT_TYPES` runtime enforcement

The auth layer rejects requests whose resolved client type isn't in the endpoint's allowed list.
For each combination, test that allowed types succeed and disallowed types return `Unauthorized`.

**Endpoints accepting `[WebDashboard, ApiToken, ServiceAccount]` (most workspace endpoints):**
- [ ] `multi_type_endpoint_accepts_jwt` — JWT works
- [ ] `multi_type_endpoint_accepts_user_api_token` — user API token works
- [ ] `multi_type_endpoint_accepts_sa_token` — SA token works

**Endpoints accepting `[WebDashboard, ApiToken]` (no SA — `create_workspace`, `user/get_user_info`,
`user/get_user_details`, `user/list_user_workspaces`, all `service_account/*`):**
- [ ] `no_sa_endpoint_rejects_sa_token` — SA token to `create_workspace` → 401
- [ ] `no_sa_endpoint_rejects_sa_token_for_sa_management` — SA token to `create_service_account`
      → 401 (specifically test the SA-managing-SA exclusion)
- [ ] `no_sa_endpoint_rejects_sa_token_for_user_endpoint` — SA token to `get_user_info` → 401
- [ ] `no_sa_endpoint_accepts_user_api_token` — user API token still works for these
- [ ] `no_sa_endpoint_accepts_jwt` — JWT still works for these

**Endpoints accepting `[WebDashboard]` only (`change_password`, `mfa/*`, `oauth/*`, etc.):**
- [ ] `web_only_endpoint_rejects_user_api_token` — user API token to `change_password` → 401
- [ ] `web_only_endpoint_rejects_sa_token` — SA token to `change_password` → 401

**Endpoints accepting `[ApiToken]` only (`docker_login`):**
- [ ] `api_token_only_endpoint_rejects_jwt` — JWT to docker_login → 401 (or not mounted)
- [ ] `api_token_only_endpoint_accepts_user_api_token` — user API token works
- [ ] `api_token_only_endpoint_rejects_sa_token` — SA token → 401

**Endpoints accepting `[ServiceAccount]` only (`stream_runner_data_for_workspace`):**
- [ ] `sa_only_endpoint_rejects_user_api_token` — user API token → 401
- [ ] `sa_only_endpoint_rejects_jwt` — JWT → 401 (or not mounted)
- [ ] `sa_only_endpoint_accepts_sa_token` — SA token works (with the right runner permission)

#### 2d. Mount-time filtering

Endpoints whose allowed types don't overlap with the server's types are not mounted at all
(returns 404, not 401). Verify both servers correctly filter.

- [ ] `web_only_endpoint_not_mounted_on_api_server` — request `change_password` on
      api.patr.cloud → 404
- [ ] `sa_only_endpoint_not_mounted_on_web_server` — request `stream_runner_data_for_workspace`
      on app.patr.cloud → 404
- [ ] `api_token_only_endpoint_not_mounted_on_web_server` — request `docker_login` on
      app.patr.cloud → 404
- [ ] `multi_type_endpoint_mounted_on_both_servers` — `create_deployment` returns 200 (or
      auth error, not 404) on both servers

#### 2e. Loki / Mimir SA-only restriction

The loki and mimir push endpoints have an explicit runtime check that
`user_data.client_type == ClientType::ServiceAccount`. Test this explicitly since it's a
hand-written check, not the auth layer's `ALLOWED_CLIENT_TYPES` mechanism.

- [ ] `loki_push_with_user_api_token_returns_403` — Basic auth with `runner_id:user_api_token`
      → 403 with body "Loki push is only allowed from service accounts"
- [ ] `loki_push_with_sa_token_succeeds` — Basic auth with `runner_id:sa_token` (where the SA
      has Runner::Execute on that runner) → 200
- [ ] `mimir_push_with_user_api_token_returns_403` — same as loki but for mimir
- [ ] `mimir_push_with_sa_token_succeeds` — same as loki but for mimir
- [ ] `loki_push_with_invalid_token_returns_401` — invalid token → 401 (existing behavior,
      regression check)

#### 2f. End-to-end SA usage flow

The motivating use case for this whole refactor: an SA in a CI pipeline calling resource
endpoints. Test the happy path end-to-end.

- [ ] `sa_can_create_deployment` — SA with `Deployment::Create` permission creates a deployment
      via api.patr.cloud → 200, deployment exists in DB
- [ ] `sa_can_list_runners` — SA with `Runner::View` lists runners → 200
- [ ] `sa_without_permission_denied_at_authorization_layer` — SA without `Deployment::Create`
      tries to create a deployment → 403 (note: this is the existing RBAC check, not the
      new ClientType check — but worth verifying it still works for SAs)
- [ ] `sa_can_manage_users_with_rbac_permission` — SA with role/user management permissions
      can call `update_user_roles_in_workspace` → 200 (validates the GCP/AWS-style decision
      to allow SA-managed IAM via RBAC)

#### 2g. Macro-level coverage

The `client_type` field is now required on every endpoint. We can't directly test proc macros
in integration tests, but we can add compile-fail tests via `trybuild` (already used? check).

- [ ] `compile_fail_missing_client_type` — trybuild test asserting an endpoint declaration
      without `client_type = [...]` fails to compile with "Missing field: client_type"
- [ ] `compile_fail_empty_client_type` — `client_type = []` fails with "client_type must
      not be empty"
- [ ] `compile_fail_unknown_variant` — `client_type = [UnknownVariant]` fails with the
      "Unknown client type" message

(Skip if trybuild isn't already a dependency — adding it for these few cases is overkill.)

#### 2h. Regression — existing tests still pass

- [ ] **Run full existing test suite** — every test in `api/tests/` should still pass after
      the refactor. The refactor preserves WebDashboard behavior since the default for
      previously-`api = true` endpoints is now `[WebDashboard, ApiToken, ServiceAccount]`
      (workspace) or `[WebDashboard, ApiToken]` (user/auth), all of which include WebDashboard.
- [ ] **Verify no endpoint silently lost WebDashboard access** — manually audit the
      `client_type` declarations to confirm WebDashboard is included where it should be.

### Notes

- Tests in 2a, 2b are foundational. Get them passing before tackling 2c onward.
- 2c is the bulk of the work. Consider parametrizing with a macro or table-driven approach
  to avoid 20+ near-identical test functions.
- 2d may overlap with 2c — a 404 on api.patr.cloud could be either "endpoint not mounted"
  or "wrong server". The error code differs, so the tests are distinct.
- 2e tests the special-cased loki/mimir auth, which doesn't use the standard auth layer.
- The trybuild tests (2g) are nice-to-have, not blocking.

### Estimated effort

- Phase 1: 1-2 days (test infra is fiddly but mechanical)
- Phase 2a-2b: half day
- Phase 2c: 1-2 days (most of the volume, parametrize aggressively)
- Phase 2d: half day
- Phase 2e: half day
- Phase 2f: half day
- Phase 2g: skip or 2 hours
- Phase 2h: half day audit
- **Total: ~5-7 days**

---

## Infrastructure / Cross-Cutting

### Concurrency & Rate Limiting

- [ ] `concurrent_create_same_resource` — race condition on unique names
- [ ] `concurrent_login_attempts` — multiple simultaneous logins
- [ ] `concurrent_token_renewal` — parallel refresh token usage

### Token & Session Handling

- [ ] `access_token_expiry_enforced` — expired access token rejected
- [ ] `refresh_token_single_use` — refresh token consumed after renewal
- [ ] `logout_invalidates_refresh_token` — currently `#[ignore]`; unblock
- [ ] `session_isolation` — user A's token cannot access user B's data

### System Endpoints

- [ ] `get_version_works` — GET `/version`

### Turnstile / Bot Protection

- [ ] `turnstile_verification_failed` — → `TurnstileVerificationFailed`
- [ ] `turnstile_action_mismatch` — → `TurnstileVerificationActionMismatch`

---

## Summary

| Module             | Existing Tests | Missing Tests | Priority     |
| ------------------ | -------------- | ------------- | ------------ |
| Auth               | 23             | 33            | High         |
| User               | 12             | 18            | High         |
| User MFA           | 3              | 6             | Medium       |
| User API Token     | 8              | 9             | Medium       |
| Workspace          | 11             | 11            | High         |
| Deployment         | 15             | 20            | High         |
| Runner             | 9              | 6             | Medium       |
| Service Account    | 17             | 6             | High         |
| Domain             | 8              | 20            | High         |
| Managed URL        | 7              | 13            | Medium       |
| Container Registry | 10             | 12            | Medium       |
| Volume             | 10             | 7             | Low          |
| Secret             | 0              | 12            | **Critical** |
| Static Site        | 0              | 17            | **Critical** |
| Database           | 0              | 12            | **Critical** |
| RBAC               | 14             | 13            | Medium       |
| Permissions        | 52             | 16            | Low          |
| Infrastructure     | 0              | 10            | Medium       |
| **Total**          | **~182**       | **~235**      |              |

---

## Registry Garbage Collection (pending GC implementation)

- [ ] `gc_cleans_orphan_blobs` — delete all manifests referencing a blob, run GC → blob removed from DB and S3
- [ ] `gc_preserves_shared_blobs` — two manifests share a layer blob, delete one → blob still exists
- [ ] `gc_cleans_orphan_manifests` — unlink manifest from repo, run GC → manifest row and S3 object removed
- [ ] `gc_cleans_orphan_config_blobs` — delete manifest → config blob orphaned → GC cleans it
