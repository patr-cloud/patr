//! Backfills the role-binding model from the legacy permission tables.
//!
//! Runs as one transaction, in this order:
//!
//! 1. One `workspace_actor` per distinct membership pair.
//! 2. Abort if any role's permissions disagree about their resources — a binding carries one scope
//!    for the whole role, so such a role has no representation and the operator has to split it
//!    first.
//! 3. Roles named after a frozen default are reset to it and marked immutable.
//! 4. `role_permission`, the flat permission list, from the legacy type rows.
//! 5. A binding per assignment: `Exclude(∅)` → one workspace-scope binding; `Include(S)` → one per
//!    live same-workspace member of S; `Exclude(S≠∅)` → one per live workspace resource not in S.
//!    Exact at cutover; resources created later are no longer auto-granted.
//! 6. Token ceilings, expanded from the legacy per-permission resource lists by those same three
//!    rules.
//!
//! Pending invites are left alone here — they only need a scope once
//! acceptance starts minting bindings, which is the next migration's job.
//!
//! The legacy tables are read throughout and left in place; dropping them is
//! a later migration, once nothing reads them.

/// One `workspace_actor` per distinct membership pair.
mod add_actors_to_table;
/// The default roles as seeded when this migration was written.
mod default_roles;
/// A binding per role assignment.
mod fill_role_bindings;
/// A role's flat permission list.
mod fill_role_permission;
/// Ceiling rows for every API token.
mod fill_user_api_token_permissions;
/// Refuses roles that have no single scope.
mod reject_non_uniform_roles;
/// Frozen defaults reset to their seeded shape.
mod reset_default_roles;

use self::{
	add_actors_to_table::add_actors_to_table,
	fill_role_bindings::fill_role_bindings,
	fill_role_permission::fill_role_permission,
	fill_user_api_token_permissions::fill_user_api_token_permissions,
	reject_non_uniform_roles::reject_non_uniform_roles,
	reset_default_roles::reset_default_roles,
};
use crate::prelude::*;

#[macros::migration]
async fn migrate(connection: &mut DatabaseConnection) -> Result<(), ErrorType> {
	add_actors_to_table(&mut *connection).await?;
	reject_non_uniform_roles(&mut *connection).await?;
	reset_default_roles(&mut *connection).await?;
	fill_role_permission(&mut *connection).await?;
	fill_role_bindings(&mut *connection).await?;
	fill_user_api_token_permissions(&mut *connection).await?;

	Ok(())
}
