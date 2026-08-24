//! Migrations for version 0.18.0

mod m001_initial_baseline;
mod m002_per_url_custom_hostnames;
mod m003_github_sso;
mod m004_sign_up_attempts;
mod m005_allow_null_manifest_config_and_platform;
mod m006_normalize_registry_schema;
mod m007_unify_domains;
mod m008_simplify_workspace_name_index;
mod m009_scope_roles_to_workspace;
mod m010_workspace_user_invites;
mod m011_rename_owner_id_to_workspace_id;
mod m012_actor_client_registry;
mod m013_audit_log_actor_client_id;
mod m014_role_permission_flat_list;
mod m015_actor;
mod m016_role_binding;
mod m017_backfill_role_bindings;
mod m018_role_binding_cutover;
