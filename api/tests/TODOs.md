# Unit Test TODOs

## Auth (`auth.rs`)

### Missing Edge Cases

- [x] `create_account_username_starts_with_dot` — rejects `.foo` (must start with `[a-z0-9_]`)
- [x] `create_account_username_ends_with_dot` — rejects `foo.` (must end with `[a-z0-9_]`)
- [x] `create_account_username_with_uppercase` — rejects `FooBar`
- [x] `create_account_invalid_email` — Phase E: handler now calls `preprocess::validators::validate_email` explicitly before falling through to availability check.
- [x] `complete_sign_up_otp_wrong_format` — non-matching `^\d{3}-?\d{3}$`

# Integration Test TODOs

Comprehensive list of missing test cases. Organized by module.

`todo!()`-stubbed modules and endpoints (OAuth, user recovery options, web logins, Secret, Static Site, Database, domain DNS records, `docker_login`) used to live here but were removed; they'll come back when the corresponding handlers land. Same for endpoints that don't exist yet (`/version`, `is_domain_personal`, `get_verification_records`).

---

## Auth (`auth.rs`)

### Missing Edge Cases

- [x] `create_account_duplicate_email` — same email, different username
- [x] `complete_sign_up_expired_otp` — OTP used after expiry window
- [x] `complete_sign_up_already_completed` — Round 3: second call returns `UserNotFound` (user_to_sign_up row deleted on success).
- [ ] `login_case_insensitive_username` — DROPPED: username regex blocks uppercase, so the test as described isn't meaningful.
- [x] `login_with_mfa_required` — returns `MfaRequired` when MFA active and OTP omitted
- [x] `login_with_mfa_valid_otp` — full MFA login flow (uses `compute_totp` helper from Round 1)
- [x] `login_with_mfa_invalid_otp` — MFA OTP wrong → `MfaOtpInvalid`
- [x] `renew_access_token_expired` — expired refresh token rejected
- [x] `forgot_password_nonexistent_user` — Phase E: handler now returns silent 202 for nonexistent users.
- [x] `forgot_password_rate_limit` — Round 3: 25 sequential calls with nonexistent user_ids (hits silent-202 path before Argon2 so they fit in the 1-second window) → at least one 429.
- [x] `reset_password_expired_otp` — OTP past expiry
- [x] `reset_password_new_password_invalid` — new password fails validation
- [x] `resend_otp_nonexistent_user` — graceful handling

---

## User (`user.rs`)

### Edge Cases

- [x] `update_user_info_empty_fields` — Round 3: PATCH with all-None succeeds and leaves fields unchanged (handler uses COALESCE).
- [x] `update_user_info_first_name_persists` — Round 3: PATCH first_name, follow-up GET confirms the change while last_name is unchanged.
- [x] `change_password_same_as_current` — Phase E: handler now rejects with `InvalidPassword` when new == current.
- [x] `change_password_new_invalid` — new password fails validation
- [x] `search_for_user_partial_match` — substring matching behavior
- [x] `search_for_user_special_chars` — SQL injection-safe search
- [x] `get_user_details_own_id` — viewing self via user_id

---

## User MFA (`user_mfa.rs`)

- [x] `activate_mfa_works` — full activation with valid TOTP code
- [x] `deactivate_mfa_works` — DELETE `/user/mfa`
- [x] `deactivate_mfa_when_inactive` — → `MfaAlreadyInactive`
- [x] `activate_mfa_when_already_active` — → `MfaAlreadyActive`
- [ ] `activate_mfa_expired_secret` — DROPPED: no expiry-on-access logic; Redis key has 5-minute TTL but the handler doesn't reject "almost expired" secrets distinctly.
- [x] `get_mfa_secret_regenerates` — calling twice gives new secret

---

## User API Token (`user_api_token.rs`)

- [x] `create_api_token_duplicate_name` — Phase E: handler now maps unique-violation on `user_api_token.name` to `ApiTokenAlreadyExists`.
- [x] `api_token_with_ip_restriction` — allowed IPs enforced
- [x] `api_token_blocked_ip` — → `DisallowedIpAddressForApiToken`
- [x] `use_api_token_for_auth` — API token in `Authorization` header works for API calls
- [x] `use_revoked_api_token` — revoked token rejected
- [x] `api_token_with_permissions` — All scoped-permission tests pass under nextest (process-per-test isolation). The earlier flakiness was a `cargo test --test-threads=1` artifact from shared in-process state.
- [~] `api_token_without_permissions` — REFRAMED: empty perms is already rejected by handler (existing `create_api_token_with_empty_permissions_fails` test). Wrote scoped-perm denial tests instead.
- [x] `update_api_token_name_conflict` — Phase E: same fix as `create_api_token_duplicate_name`.
- [x] `list_api_tokens_pagination` — verify ordering/limits

---

## Workspace (`workspace.rs`)

- [ ] `delete_workspace_works` — currently `#[ignore]` due to audit_log FK; unblock and test
- [ ] `delete_workspace_not_empty` — workspace with resources → `WorkspaceNotEmpty`
- [ ] `delete_workspace_with_deployments` — FK constraint blocks delete
- [ ] `delete_workspace_with_volumes` — FK constraint blocks delete
- [ ] `delete_workspace_with_domains` — FK constraint blocks delete
- [x] `create_workspace_name_too_short` — < 4 chars rejected (`RESOURCE_NAME_REGEX`)
- [x] `create_workspace_name_too_long` — > 255 chars rejected
- [x] `create_workspace_name_special_chars` — chars outside `[a-zA-Z0-9\-_ .]` rejected
- [x] `update_workspace_name_conflict` — rename to taken name → `WorkspaceNameAlreadyExists`
- [x] `update_workspace_unauthorized` — non-member cannot update
- [x] `list_user_workspaces_multiple` — user in multiple workspaces (test lives in `user/mod.rs` since the endpoint is `/user/workspaces`)

---

## Deployment (`workspace_deployment.rs`)

- [x] `create_deployment_duplicate_name` — same name in workspace → `ResourceAlreadyExists`
- [ ] `create_deployment_invalid_machine_type` — DEFERRED: handler doesn't FK-check machine_type; would need a paired handler fix.
- [x] `create_deployment_with_volumes` — attach volumes on create
- [x] `create_deployment_with_env_vars` — environment variables
- [x] `create_deployment_with_ports` — port configuration
- [x] `update_deployment_name` — Round 2: added `update_deployment_name_persists` that verifies the rename via a follow-up GET (the existing `update_deployment_works` only asserts the call returned success).
- [x] `update_deployment_machine_type` — change machine type
- [x] `update_deployment_image` — Editing the image tag is now supported (`image_tag` on `UpdateDeploymentRequest`). Added `update_deployment_image_tag_persists` (external registry: tag persists via a follow-up GET), `update_deployment_invalid_tag_400` and `update_deployment_empty_tag_400` (tag regex rejects bad/empty tags). TODO: `update_deployment_patr_tag_resolves_digest` — for a Patr-registry deployment, changing the tag should re-resolve `current_live_digest` and write a deploy-history row; deferred because it needs container-registry repository + manifest-tag seeding infra that no test helper covers yet (repos are created via the docker push flow). Covered manually via E2E for now.
- [x] `start_deployment_already_running` — REFRAMED: handler is idempotent (`start_deployment.rs:136-151` unconditionally sets status); added `start_deployment_idempotent`.
- [x] `stop_deployment_already_stopped` — REFRAMED: same; added `stop_deployment_idempotent`.
- [x] `delete_deployment_while_running` — Round 2: handler doesn't check status; test starts the deployment then deletes, asserts success.
- [x] `get_deployment_logs_empty` — no logs yet (returns 200 with empty array)
- [x] `get_deployment_metric_empty` — Round 2: added a Mimir testcontainer to `setup.rs` (`grafana/mimir:2.13.0`, monolithic in-memory config). Existing `get_deployment_metric_works` was tightened from "200 or 5xx" to "200".
- [x] `deployment_cross_workspace` — deployment in workspace A not accessible from workspace B

### Deploy History

- [x] `delete_deploy_history_works` — Round 3: seed via `execute_sql`, delete by digest, verify gone via list.
- [x] `delete_deploy_history_nonexistent` — invalid digest → `ResourceDoesNotExist`
- [x] `revert_deployment_works` — POST `.../deploy-history/{digest}/revert`
- [x] `revert_deployment_nonexistent_digest` — bad digest → `ResourceDoesNotExist`
- [x] `revert_deployment_to_current` — handler unconditionally re-applies the digest; no-op success.
- [x] `list_deploy_history_after_multiple_deploys` — ordered by `created DESC`. Also tightened the existing `list_deploy_history_empty` from "200 or 5xx" to "200 with empty deploys".

---

## Runner (`workspace_runner.rs`)

- [x] `add_runner_duplicate_name` — Round 2: paired handler fix in `add_runner_to_workspace.rs`. The `runner` insert wasn't catching unique violations (only the `resource` insert was) so duplicate names returned a 500.
- [x] `add_runner_invalid_name` — name outside `RESOURCE_NAME_REGEX`
- [ ] `runner_already_connected` — needs WebSocket connection state; deferred to a runner-WS test round.
- [ ] `runner_invalid_mode` — needs WebSocket connection state; deferred.
- [x] `get_ingress_token_nonexistent_runner` — → `ResourceDoesNotExist`
- [x] `runner_cross_workspace` — runner in workspace A not visible from B
- [ ] `get_runner_logs_*` — handler exists (`get_runner_logs.rs`) and is real; zero tests today.
- [ ] `get_runner_metrics_*` — handler exists (`get_runner_metrics.rs`) and is real; zero tests today.

---

## Domain (`workspace_domain.rs`)

### Verification

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
- [ ] `verify_domain_works` — POST `.../domain/{id}/verify` against a domain with the correct TXT record set up (currently bypassed in tests by flipping `is_verified` via `TestSetup::mark_test_domain_verified`)
- [ ] `verify_domain_already_verified` — handler is real; double-verify behavior currently untested
- [ ] `verify_domain_unverifiable` — DNS not configured
- [ ] `verify_domain_wrong_txt_value` — TXT record present but value doesn't match

### Edge Cases

- [x] `add_domain_not_root` — → `NotRootDomain` (subdomains rejected before insert)
- [x] `add_domain_not_icann` — → `NotIcannDomain` (`.local` is in PSL private section)
- [x] `add_domain_duplicate` — → `ResourceAlreadyExists`
- [x] `delete_domain_in_use` — domain with attached managed URL → `ResourceInUse`
- [x] `domain_cross_workspace` — domain in workspace A not reachable via workspace B's path

---

## Managed URL (`workspace_managed_url.rs`)

- [x] `create_managed_url_proxy_deployment` — Round 2: deployment must declare the port (FK `managed_url_fk_deployment_id_port`); test inlines a deployment with port 8080 exposed.
- [x] `create_managed_url_proxy_url` — type `ProxyUrl` with `url` + `http_only`
- [x] `create_managed_url_redirect` — covers both `permanent_redirect` true and false in one test
- [x] `create_managed_url_redirect_permanent` — covered by `create_managed_url_redirect` (loops over both values)
- [x] `create_managed_url_invalid_deployment_id` — nonexistent deployment → `WrongParameters`
- [x] `create_managed_url_unverified_domain` — `DomainNotVerified` (`create_managed_url.rs:74-76`)
- [x] `update_managed_url_change_type` — Round 3: covered by `update_managed_url_change_redirect_to_proxy_url` and `update_managed_url_change_proxy_url_to_redirect`.
- [x] `get_managed_url_info` — Round 3: `get_managed_url_info_via_list` verifies the list endpoint returns sub_domain, domain_id, path, and full url_type-specific fields.
- [x] `delete_managed_url_nonexistent` — → `ResourceDoesNotExist`
- [x] `verify_configuration_not_configured` — Round 3: added a high-priority wiremock route in setup.rs that returns `status: "pending"` for custom hostname id `pending-hostname-id`. Test re-points the managed_url_custom_hostname row at that id, calls verify, asserts `configured = false`.
- [x] `managed_url_cross_workspace` — URL in workspace A not reachable via workspace B's path

---

## Container Registry (`workspace_container_registry.rs`)

### Manifest Endpoints

- [x] `get_manifest_details_works` — Round 3: paired model fix in `get_repository_manifest_details.rs` (added `#[serde(default)]` to `referenced_manifests`; round-tripping was broken).
- [x] `get_manifest_details_nonexistent` — → `ResourceDoesNotExist`
- [x] `get_exposed_ports_works` — Round 3: paired handler fix in `get_exposed_ports.rs`. Was parsing the OCI config blob as `Config` (the inner runtime config) instead of `ImageConfiguration`; now drills into `.config().exposed_ports()` and parses `port/tcp` form.
- [x] `get_exposed_ports_no_ports` — image with no EXPOSE
- [x] `delete_manifest_works` — DELETE `.../manifest/{digest_or_tag}`
- [x] `delete_manifest_nonexistent` — → `ResourceDoesNotExist`

### Push/Pull Flow

- [ ] `push_image_and_list_tags` — push via Docker, verify tags appear (overlap with existing `tests/registry/push_pull.rs`; needs cross-check before adding)
- [ ] `push_image_and_list_manifests` — same
- [ ] `push_multiple_tags` — same image, multiple tags
- [x] `delete_tag_in_use` — Round 3: REFRAMED as `delete_manifest_in_use_by_deployment` (the `delete_repository_manifest` endpoint is the only delete path here). Paired handler fix added an in-use check that refuses delete with `ResourceInUse` when any live deployment in the workspace references a tag pointing at the manifest.

### Edge Cases

- [x] `create_repository_invalid_name` — name outside `RESOURCE_NAME_REGEX`
- [x] `delete_repository_with_images` — REFRAMED: handler explicitly cascades manifests/tags on delete (`delete_repository.rs:56-79`), so "not empty" doesn't block. The actual `ResourceInUse` path is when a deployment references the repo. Round 2 added `delete_repository_in_use_by_deployment` to cover that.
- [x] `container_registry_cross_workspace` — repo in workspace A not reachable via workspace B's path

---

## Volume (`workspace_volume.rs`)

- [x] `create_volume_name_too_short` — < 4 chars
- [x] `create_volume_name_too_long` — > 255 chars
- [x] `update_volume_increase_size` — size increase accepted
- [x] `update_volume_decrease_size` — → `CannotReduceVolumeSize`
- [x] `delete_volume_attached_to_deployment` — → `ResourceInUse`
- [ ] `create_volume_exceeds_limit` — DROPPED: handler `create_volume.rs` doesn't enforce a per-workspace volume cap.
- [x] `volume_cross_workspace` — volume in workspace A not visible from B

---

## RBAC (`workspace_rbac.rs`)

- [x] `create_role_duplicate_name` — → `RoleAlreadyExists`
- [x] `create_role_invalid_name` — name outside `RESOURCE_NAME_REGEX`
- [x] `delete_role_in_use` — role assigned to users → `RoleInUse`
- [x] `delete_role_nonexistent` — Phase E: handler now checks rowcount → `RoleDoesNotExist`
- [x] `update_role_nonexistent` — Phase E: same
- [x] `update_role_add_permissions` — add permissions to existing role
- [x] `update_role_remove_permissions` — remove permissions from existing role
- [x] `update_user_roles_nonexistent_user` — Phase E: handler maps FK violation → `UserNotFound`
- [x] `update_user_roles_nonexistent_role` — Phase E: handler maps FK violation on role_id → `RoleDoesNotExist`
- [x] `remove_user_from_workspace_not_member` — Phase E: handler now returns `UserNotFound` when 0 rows removed
- [ ] `remove_self_from_workspace` — super admin removing self
- [x] `add_user_to_workspace_already_member` — REFRAMED: there's no dedicated `add_user` endpoint. Membership is via `update_user_roles_in_workspace`. Round 2 added `update_user_roles_idempotent` which calls it twice with the same role and asserts no error.
- [x] `list_users_for_role_empty` — already covered by existing `list_users_for_role_works` (asserts `users.is_empty()` for a fresh role).

---

## Permissions (`permissions.rs`)

### Include/Exclude

- [x] `volume_include_specific` — already covered by existing `volume_include_grants_only_listed_resource`
- [x] `domain_include_specific` — already covered by existing `domain_include_grants_only_listed_resource`
- [x] `container_registry_include_specific` — Phase F: added `container_registry_view_include_grants_only_listed_resource` (also pre-existing `container_registry_delete_include_grants_only_listed_resource`)
- [x] `managed_url_include_specific` — Phase F: added `managed_url_delete_include_grants_only_listed_resource`
- [x] `runner_include_specific` — already covered by existing `runner_include_grants_only_listed_resource` (currently affected by `GetRunnerInfo` 500 bug — see runner.rs ignored test note)
- [x] `deployment_exclude_specific` — already covered by existing `deployment_exclude_denies_only_listed_resource`
- [x] `volume_exclude_specific` — already covered by existing `volume_exclude_denies_only_listed_resource`
- [x] `runner_exclude_specific` — already covered by existing `runner_exclude_denies_only_listed_resource` (also affected by GetRunnerInfo bug)
- [x] `domain_exclude_specific` — already covered by existing `domain_exclude_denies_only_listed_resource`
- [x] `container_registry_exclude_specific` — Phase F: added `container_registry_view_exclude_denies_only_listed_resource` and `managed_url_delete_exclude_denies_only_listed_resource`

### Cross-Permission

- [x] `volume_view_doesnt_grant_delete` — Phase F: needed `GetVolumeInfo` endpoint fix (was requiring `Delete` instead of `View`).
- [x] `volume_view_doesnt_grant_edit` — Phase F: same fix.
- [x] `domain_view_doesnt_grant_delete` — Phase F.
- [x] `container_registry_view_doesnt_grant_delete` — Phase F: needed `GetContainerRepositoryInfo` extractor fix (was extracting `workspace_id` instead of `repository_id`).
- [x] `managed_url_view_doesnt_grant_delete` — Phase F.
- [x] `runner_view_doesnt_grant_create` — Fixed root cause: `get_runner_info` was using `SELECT *` which the sqlx macro validated against a live DB schema with extra columns (`version`, `service_account_id`) not in the test DB. Changed to explicit `SELECT name`. Also raised crate `recursion_limit` to 256 to match what the compiler asks for.

### API Token Permission

- [ ] `api_token_with_workspace_permissions` — token with scoped workspace access (likely already covered by Round 1 scoped-perm tests; needs cross-check)
- [ ] `api_token_denied_without_permission` — same
- [ ] `api_token_resource_level_permissions` — same

---

## Infrastructure / Cross-Cutting

### Concurrency & Rate Limiting

- [x] `concurrent_create_same_resource` — Round 3: paired handler fix in `create_workspace.rs`. The workspace INSERT didn't map unique violations (only the resource INSERT did) so concurrent same-name calls leaked some 500s. Fixed → `WorkspaceNameAlreadyExists`. 5 concurrent → 1 success, 4 client errors.
- [ ] `concurrent_login_attempts` — DROPPED: login is naturally idempotent; unclear what to assert.
- [x] `concurrent_token_renewal` — Round 3: paired handler fix in `renew_access_token.rs`. Added `SELECT ... FOR UPDATE` on the `web_login` row so verify+rotate is atomic. Without this, two concurrent renews both verified the same hash before either rotated, defeating single-use. Test now: 2 concurrent renews → 1 success + 1 client error.

### Token & Session Handling

- [x] `access_token_expiry_enforced` — Round 2: backdates `web_login.token_expiry` via `execute_sql`. The JWT `exp` claim is signed and unforgeable, but the auth layer (`web_dashboard.rs:109`) re-checks the DB row, which is what we kill.
- [x] `refresh_token_single_use` — Round 3: paired handler change in `renew_access_token.rs` rotates the refresh token on every call (generates new token, replaces hashed value, returns the new token in the response). Frontend updated in `frontend/src/utils/http-request.ts` to capture the new refresh token. Test asserts old token is rejected after first renew while new token works.
- [ ] `logout_invalidates_refresh_token` — currently `#[ignore]`; unblock (extractor design conflict between `PlainTokenAuthenticator` and `BearerToken`).
- [x] `session_isolation` — two users get distinct identities back from `/user`; tokens don't cross-resolve.

### Turnstile / Bot Protection

- [ ] `turnstile_verification_failed` — DROPPED: needs per-test override of `cloudflare.turnstile_secret` to the always-fail dummy (`2x0000000000000000000000000000000AA`). Test secret is the always-pass one, so siteverify returns success=true regardless of token.
- [ ] `turnstile_action_mismatch` — DROPPED: Cloudflare test tokens don't expose action-mismatch as documented behaviour.

---

## Registry Garbage Collection (pending GC implementation)

- [ ] `gc_cleans_orphan_blobs` — delete all manifests referencing a blob, run GC → blob removed from DB and S3
- [ ] `gc_preserves_shared_blobs` — two manifests share a layer blob, delete one → blob still exists
- [ ] `gc_cleans_orphan_manifests` — unlink manifest from repo, run GC → manifest row and S3 object removed
- [ ] `gc_cleans_orphan_config_blobs` — delete manifest → config blob orphaned → GC cleans it

---

# Runner Integration Test Strategy (not yet implemented)

**Current state:** no test infrastructure exists for the Docker or Kubernetes runners. All verification is manual.

The runner exercises three external surfaces: Docker Engine (Swarm API via bollard), SQLite, and the upstream Patr API over WebSocket. Swarm semantics (immutable configs, "config in use can't be deleted", rolling updates, label filters) can't be meaningfully mocked — a fake bollard just tests the fake.

## Test levels, ranked by value per effort

1. **Executor-level integration tests against a real Docker Swarm.** Construct `DockerRunner { docker, settings }` manually with self-hosted mode and in-memory SQLite, call `upsert_deployment` / `delete_deployment` / `update_alloy_service` directly, assert Docker state via the same bollard client. Highest coverage-per-LOC. Where most of the value is.
2. **Unit tests** for pure logic: `derive_base_name` parsers, ordinal-from-label extraction, hash-length growing, label merging. Cheap, catches regressions in the fiddly bits, but there isn't much pure logic in the runner.
3. **End-to-end** with a real API server + runner binary over WebSocket. Most comprehensive, slowest, most brittle. Reserve for specific cross-cutting flows (e.g. "deploy-on-push triggers runner upsert within N seconds").

Aim for (1) as the backbone, (2) where the logic warrants it, skip (3) until a regression makes the case.

## Practical stack for (1)

`testcontainers` already handles Postgres/Redis/MinIO in the API tests (`api/tests/setup.rs`). Same library has a Docker-in-Docker image. Per test (or per test module), spin up a fresh DinD:

```rust
use testcontainers::{GenericImage, runners::AsyncRunner};

let dind = GenericImage::new("docker", "24-dind")
    .with_exposed_port(2375.tcp())
    .with_env_var("DOCKER_TLS_CERTIFIED", "")
    .start()
    .await?;
let host_port = dind.get_host_port_ipv4(2375).await?;
let docker = Docker::connect_with_http(&format!("tcp://127.0.0.1:{host_port}"), ...)?;
docker.init_swarm(SwarmInitRequest::default()).await?;
```

Each test gets its own Swarm → full isolation, no cross-test leakage, parallelism capped only by how many DinDs the host can run.

## Isolation strategies

- **Per-test DinD** — strongest isolation, slowest (10-20s per test). Use for destructive scenarios: migration script behavior, swarm-init semantics.
- **Shared DinD per module, unique resource names per test** — faster, but names must include a test-scoped prefix (ULID or similar) to avoid collisions on Swarm-wide name uniqueness.
- **Host Docker** — local dev iteration only (`cargo test -- --ignored`). Never in CI; would mutate the dev machine.

## Covering the upstream API dependency

The runner's managed-mode path connects over WebSocket via `client::make_request`. Two options:

- **Stub WebSocket server** (`tokio-tungstenite` fixture) — fine-grained, no API surface area required.
- **Real API in another testcontainer** — closer to prod, much more setup.

For Docker-side logic, **self-hosted mode** bypasses the API entirely — use that for the bulk of tests. A small handful of managed-mode tests covers the WebSocket-message-to-DB path.

## Proposed test organization

```
runners/docker/tests/
├── common/
│   ├── mod.rs           # DinD + DockerRunner fixtures
│   └── fixtures.rs      # deployment, mount, service builders
├── config_mounts.rs     # upsert/update/remove/delete lifecycle
├── label_migration.rs   # backfill script + update_config early-return
├── ingress.rs           # Caddy config aggregation, tunnel-token
└── alloy.rs             # Alloy service spec
```

Fixture provides:

```rust
struct TestRunner {
    docker: Docker,                        // points at DinD
    runner: DockerRunner,
    db: SqlitePool,
    _dind: ContainerAsync<GenericImage>,   // drops → cleanup
}

impl TestRunner {
    async fn new() -> Self { ... }
    async fn create_deployment(&self, mounts: Vec<(&str, &str)>) -> Uuid { ... }
    async fn configs_for(&self, deployment_id: Uuid) -> Vec<ConfigInfo> { ... }
}
```

## Initial smoke suite

Three tests covering the highest-impact scenarios:

- [ ] `upsert_deployment_creates_mount_configs_and_service_references_them` — baseline happy path.
- [ ] `updating_mount_content_creates_new_config_and_cleans_up_old` — exercises `update_config`'s hash-match + cleanup.
- [ ] `deleting_deployment_removes_service_and_all_its_configs` — lifecycle termination.

Grow the suite by adding one test per regression. Don't try to cover everything up front.

## What to skip

- **Mocking `bollard`** — tests the mock, not the behavior.
- **Mocking the `Docker` struct** — bollard's types aren't trait-based; mocking means reimplementing half of Docker.
- **Golden-file assertions on serialized specs** — Docker rewrites fields on roundtrip, makes comparisons brittle.
- **Testing startup ordering through a full binary** — a runtime assertion (panic if `E::initialize` runs before `db::initialize`) catches regressions for free.

## Cost expectations

DinD tests aren't free. A realistic suite of ~20 tests would take 3-5 minutes locally, longer in CI on modest runners. Parallelize across test modules but serialize within one (Swarm init is the bottleneck). Cap container count with `RUST_TEST_THREADS=N`.
