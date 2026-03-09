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
