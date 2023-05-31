use api_macros::{query, query_as};
use api_models::{
	models::workspace::ci::{
		git_provider::BuildStatus,
		runner::{Runner, RunnerBuildDetails},
	},
	utils::{self, Uuid},
};
use futures::TryStreamExt;

use super::WorkspaceRepository;
use crate::Database;

pub struct RunnerResource {
	cpu: u32,
	ram: u32,
	volume: u32,
}

impl RunnerResource {
	pub fn cpu_in_milli(&self) -> u32 {
		self.cpu * 1000
	}

	pub fn ram_in_mb(&self) -> u32 {
		self.ram * 250
	}

	pub fn volume_in_mb(&self) -> u32 {
		self.volume * 1000
	}
}

pub async fn initialize_ci_runner_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing ci runner tables");

	query!(
		r#"
		CREATE TABLE ci_runner (
			id              		UUID			NOT NULL,
			name            		CITEXT			NOT NULL,
			workspace_id    		UUID    		NOT NULL,
			region_id       		UUID    		NOT NULL,
			build_machine_type_id	UUID			NOT NULL,
			deleted					TIMESTAMPTZ,

			CONSTRAINT ci_runner_pk PRIMARY KEY (id),

			CONSTRAINT ci_runner_fk_workspace_id
				FOREIGN KEY (workspace_id) REFERENCES workspace(id),
			CONSTRAINT ci_runner_fk_region_id
				FOREIGN KEY (region_id) REFERENCES region(id),
			CONSTRAINT ci_runner_fk_build_machine_type_id
				FOREIGN KEY (build_machine_type_id) REFERENCES ci_build_machine_type(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX ci_runner_uq_workspace_id_name
			ON ci_runner (workspace_id, name)
				WHERE deleted IS NULL;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_ci_runner_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up ci tables initialization");

	query!(
		r#"
		ALTER TABLE ci_runner
		ADD CONSTRAINT ci_runner_fk_id_workspace_id
		FOREIGN KEY(id, workspace_id) REFERENCES resource(id, owner_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn get_runners_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<Runner>, sqlx::Error> {
	query_as!(
		Runner,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			region_id as "region_id: _",
			build_machine_type_id as "build_machine_type_id: _"
		FROM ci_runner
		WHERE
			workspace_id = $1 AND
			deleted IS NULL;
		"#,
		workspace_id as _
	)
	.fetch_all(connection)
	.await
}

pub async fn create_runner_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	runner_id: &Uuid,
	name: &str,
	workspace_id: &Uuid,
	region_id: &Uuid,
	build_machine_type_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO ci_runner (
			id,
			name,
			workspace_id,
			region_id,
			build_machine_type_id
		)
		VALUES ($1, $2, $3, $4, $5);
		"#,
		runner_id as _,
		name as _,
		workspace_id as _,
		region_id as _,
		build_machine_type_id as _,
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn get_runner_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	runner_id: &Uuid,
) -> Result<Option<Runner>, sqlx::Error> {
	query_as!(
		Runner,
		r#"
		SELECT
			id as "id: _",
			name::TEXT as "name!: _",
			region_id as "region_id: _",
			build_machine_type_id as "build_machine_type_id: _"
		FROM ci_runner
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		runner_id as _,
	)
	.fetch_optional(connection)
	.await
}

pub async fn update_runner(
	connection: &mut <Database as sqlx::Database>::Connection,
	runner_id: &Uuid,
	name: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE ci_runner
		SET name = $2
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		runner_id as _,
		name as _,
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn list_active_repos_for_runner(
	connection: &mut <Database as sqlx::Database>::Connection,
	runner_id: &Uuid,
) -> Result<Vec<WorkspaceRepository>, sqlx::Error> {
	query_as!(
		WorkspaceRepository,
		r#"
		SELECT
			repo_owner,
			repo_name,
			clone_url,
			ci_repos.git_provider_id as "git_provider_id: _",
			git_provider_repo_uid,
			ci_workspace_repos.runner_id as "runner_id: _",
			ci_workspace_repos.workspace_id as "workspace_id: _",
			resource_id as "resource_id: _",
			activated
		FROM
			ci_repos
		LEFT JOIN
			ci_workspace_repos
		ON
			ci_workspace_repos.git_repo_id = ci_repos.git_provider_repo_uid
		WHERE
			ci_workspace_repos.runner_id = $1 AND
			ci_workspace_repos.activated = true;
		"#,
		runner_id as _,
	)
	.fetch_all(connection)
	.await
}

pub async fn mark_runner_as_deleted(
	connection: &mut <Database as sqlx::Database>::Connection,
	runner_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query_as!(
		Runner,
		r#"
		UPDATE ci_runner
		SET deleted = NOW()
		WHERE
			id = $1 AND
			deleted IS NULL;
		"#,
		runner_id as _,
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn list_build_details_for_runner(
	connection: &mut <Database as sqlx::Database>::Connection,
	runner_id: &Uuid,
) -> Result<Vec<RunnerBuildDetails>, sqlx::Error> {
	query!(
		r#"
		SELECT
			ci_workspace_repos.git_repo_id as "github_repo_id",
			ci_builds.build_num,
			ci_builds.git_ref,
			ci_builds.git_commit,
			ci_builds.status as "status: BuildStatus",
			ci_builds.created,
			ci_builds.started,
			ci_builds.finished,
			ci_builds.message,
			ci_builds.author,
			ci_builds.git_pr_title,
			ci_builds.git_commit_message
		FROM
			ci_builds
		JOIN ci_workspace_repos
			ON ci_workspace_repos.resource_id = ci_builds.repo_id
		WHERE
			ci_builds.runner_id = $1
		ORDER BY ci_builds.created DESC;
		"#,
		runner_id as _,
	)
	.fetch(&mut *connection)
	.map_ok(|build| RunnerBuildDetails {
		github_repo_id: build.github_repo_id,
		build_num: build.build_num as u64,
		git_ref: build.git_ref,
		git_commit: build.git_commit,
		status: build.status,
		created: utils::DateTime(build.created),
		started: build.started.map(utils::DateTime),
		finished: build.finished.map(utils::DateTime),
		message: build.message,
		author: build.author,
		git_pr_title: build.git_pr_title,
		git_commit_message: build.git_commit_message,
	})
	.try_collect()
	.await
}

pub async fn list_queued_builds_for_runner(
	connection: &mut <Database as sqlx::Database>::Connection,
	runner_id: &Uuid,
) -> Result<Vec<RunnerBuildDetails>, sqlx::Error> {
	query!(
		r#"
		SELECT
			ci_workspace_repos.git_repo_id as "github_repo_id",
			ci_builds.build_num,
			ci_builds.git_ref,
			ci_builds.git_commit,
			ci_builds.status as "status: BuildStatus",
			ci_builds.created,
			ci_builds.started,
			ci_builds.finished,
			ci_builds.message,
			ci_builds.author,
			ci_builds.git_pr_title,
			ci_builds.git_commit_message
		FROM
			ci_builds
		JOIN ci_workspace_repos
			ON ci_workspace_repos.resource_id = ci_builds.repo_id
		WHERE
			ci_builds.runner_id = $1 AND
			(
				ci_builds.status = 'waiting_to_start' OR
				ci_builds.status = 'running'
			)
		ORDER BY ci_builds.created ASC;
		"#,
		runner_id as _,
	)
	.fetch(&mut *connection)
	.map_ok(|build| RunnerBuildDetails {
		github_repo_id: build.github_repo_id,
		build_num: build.build_num as u64,
		git_ref: build.git_ref,
		git_commit: build.git_commit,
		status: build.status,
		created: utils::DateTime(build.created),
		started: build.started.map(utils::DateTime),
		finished: build.finished.map(utils::DateTime),
		message: build.message,
		author: build.author,
		git_pr_title: build.git_pr_title,
		git_commit_message: build.git_commit_message,
	})
	.try_collect()
	.await
}

pub async fn get_runner_resource_for_build(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
	build_num: i64,
) -> Result<Option<RunnerResource>, sqlx::Error> {
	query!(
		r#"
		SELECT
			cpu,
			ram,
			volume
		FROM
			ci_build_machine_type
		JOIN ci_runner
			ON ci_runner.build_machine_type_id = ci_build_machine_type.id
		JOIN ci_builds
			ON ci_builds.runner_id = ci_runner.id
		WHERE (
			ci_builds.repo_id = $1 AND
			ci_builds.build_num = $2
		);
		"#,
		repo_id as _,
		build_num,
	)
	.fetch_optional(connection)
	.await
	.map(|op| {
		op.map(|record| RunnerResource {
			cpu: record.cpu as u32,
			ram: record.ram as u32,
			volume: record.volume as u32,
		})
	})
}

pub async fn is_runner_available_to_start_build(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
	build_num: i64,
) -> Result<bool, sqlx::Error> {
	query!(
		r#"
		SELECT
			(repo_id = $1 AND build_num = $2) AS "available"
		FROM
			ci_builds
		WHERE (
			runner_id = (
				SELECT runner_id
				FROM ci_builds
				WHERE (
					repo_id = $1
					AND build_num = $2
				)
			) AND
			(
				status = 'waiting_to_start' OR
				status = 'running'
			)
		)
		ORDER BY created ASC
		LIMIT 1;
		"#,
		repo_id as _,
		build_num,
	)
	.fetch_optional(connection)
	.await
	.map(|optional_record| {
		optional_record
			.and_then(|record| record.available)
			.unwrap_or(false)
	})
}
