use std::collections::{BTreeMap, BTreeSet};

use models::rbac::WorkspacePermission;
use sqlx::Connection as _;
use time::{Duration, OffsetDateTime};

use crate::{
	models::redis::{ActorAuthDataCache, ActorAuthDataCacheKind},
	prelude::*,
};

/// Load everything the cache holds for the runner whose token hashes to
/// `token_hash`: the runner itself and its permissions. A runner holds a
/// single credential, so it is its own login ID and its own actor ID.
///
/// A runner isn't a workspace actor and holds no role bindings: its
/// permissions are fixed here, the same for every runner. It reads what it
/// needs to run deployments across its own workspace, and views and edits
/// itself. Edit is what the routes only a runner may call require.
/// Nothing grantable changes this map.
pub(super) async fn load_actor_auth_data(
	database: &mut DatabaseConnection,
	token_hash: &str,
) -> Result<(ActorAuthDataCache, Duration), ErrorType> {
	// Taken before the lookup, so a stamp written while the lookup is in
	// flight still marks this entry stale.
	let created_at = OffsetDateTime::now_utc();

	// Waits out an uncommitted regenerate or delete of the row, then releases
	// the lock right away so a long request doesn't hold it.
	let mut savepoint = database.begin().await?;
	let runner = query!(
		r#"
		SELECT
			runner.id,
			runner.workspace_id,
			runner.name,
			resource.created
		FROM
			runner
		INNER JOIN
			resource
		ON
			resource.id = runner.id
		WHERE
			runner.token_hash = $1 AND
			runner.deleted IS NULL
		FOR SHARE OF runner;
		"#,
		token_hash,
	)
	.fetch_optional(&mut *savepoint)
	.await?;
	savepoint.rollback().await?;

	let Some(runner) = runner else {
		warn!("No live runner with this token hash");
		return Err(ErrorType::AuthorizationTokenInvalid);
	};

	let mut permissions = BTreeMap::<Uuid, BTreeSet<Uuid>>::new();
	for permission in [
		Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::View),
		Permission::ContainerRegistryRepository(ContainerRegistryRepositoryPermission::Pull),
		Permission::Deployment(DeploymentPermission::View),
		Permission::Domain(DomainPermission::View),
		Permission::ManagedURL(ManagedURLPermission::View),
		Permission::Secret(SecretPermission::View),
	] {
		permissions.insert(
			super::get_permission_id(&mut *database, permission).await,
			BTreeSet::from([runner.workspace_id]),
		);
	}
	for permission in [
		Permission::Runner(RunnerPermission::View),
		Permission::Runner(RunnerPermission::Edit),
	] {
		permissions.insert(
			super::get_permission_id(&mut *database, permission).await,
			BTreeSet::from([runner.id]),
		);
	}

	Ok((
		ActorAuthDataCache {
			login_id: runner.id,
			actor_id: runner.id,
			kind: ActorAuthDataCacheKind::Runner {
				name: runner.name,
				created: runner.created,
			},
			permissions: BTreeMap::from([(
				runner.workspace_id,
				WorkspacePermission::Member { permissions },
			)]),
			created_at,
		},
		constants::CACHED_PERMISSIONS_VALIDITY,
	))
}
