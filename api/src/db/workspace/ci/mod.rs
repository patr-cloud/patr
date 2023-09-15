mod runner;

use api_macros::query;
use api_models::{
	models::workspace::ci::{
		get_recent_activity::RecentActivity,
		git_provider::{
			BuildDetails,
			BuildStatus,
			BuildStepStatus,
			GitProviderType,
			RepoStatus,
			RepositoryDetails,
			Step,
		},
		BuildMachineType,
	},
	utils::{self, Uuid},
};
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use sqlx::query_as;

pub use self::runner::*;
use crate::{db, Database};

pub struct GitProvider {
	pub id: Uuid,
	pub workspace_id: Uuid,
	pub domain_name: String,
	pub git_provider_type: GitProviderType,
	pub login_name: Option<String>,
	pub password: Option<String>,
	pub is_syncing: bool,
	pub last_synced: Option<DateTime<Utc>>,
	pub is_deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Repository {
	pub id: Uuid,
	pub repo_owner: String,
	pub repo_name: String,
	pub clone_url: String,
	pub webhook_secret: Option<String>,
	pub status: RepoStatus,
	pub git_provider_id: Uuid,
	pub git_provider_repo_uid: String,
	pub runner_id: Option<Uuid>,
}

pub struct BuildRecord {
	pub build_num: i64,
	pub git_ref: String,
	pub git_commit: String,
	pub author: Option<String>,
	pub status: BuildStatus,
	pub created: DateTime<Utc>,
	pub started: Option<DateTime<Utc>>,
	pub finished: Option<DateTime<Utc>>,
	pub message: Option<String>,
	pub git_commit_message: Option<String>,
	pub git_pr_title: Option<String>,
	pub runner_id: Uuid,
}

pub struct StepRecord {
	pub step_id: i32,
	pub step_name: String,
	pub base_image: String,
	pub commands: String,
	pub status: BuildStepStatus,
	pub started: Option<DateTime<Utc>>,
	pub finished: Option<DateTime<Utc>>,
}

pub async fn initialize_ci_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Initializing ci tables");

	query!(
		r#"
		CREATE TABLE ci_build_machine_type(
			id 		UUID CONSTRAINT ci_machint_type_pk PRIMARY KEY,
			cpu 	INTEGER NOT NULL, /* Multiples of 1 vCPU */
			ram 	INTEGER NOT NULL, /* Multiples of 0.25 GB RAM */
			volume 	INTEGER NOT NULL, /* Multiples of 1 GB storage space */

			CONSTRAINT ci_build_machine_type_chk_cpu_positive CHECK(cpu > 0),
			CONSTRAINT ci_build_machine_type_chk_ram_positive CHECK(ram > 0),
			CONSTRAINT ci_build_machine_type_chk_volume_positive
				CHECK(volume > 0)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	runner::initialize_ci_runner_tables(connection).await?;

	query!(
		r#"
		CREATE TYPE CI_GIT_PROVIDER_TYPE AS ENUM(
			'github'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	/* NOTE: do we need to support clientId and clientSecret for oauth nowitself?
	client_id TEXT,
	client_secret TEXT,

	CONSTRAINT ci_git_provider_ch_login_name_password
		CHECK (
			(client_id IS NULL AND client_secret IS NULL)
			OR (client_id IS NOT NULL AND client_secret IS NOT NULL)
		),
	*/
	query!(
		r#"
		CREATE TABLE ci_git_provider(
			id 					UUID CONSTRAINT ci_git_provider_pk PRIMARY KEY,
			workspace_id 		UUID NOT NULL
				CONSTRAINT ci_git_provider_fk_workspace_id
					REFERENCES workspace(id),
			domain_name 		TEXT NOT NULL,
			git_provider_type 	CI_GIT_PROVIDER_TYPE NOT NULL,
			login_name 			TEXT,
			password 			TEXT,
			is_deleted			BOOL NOT NULL DEFAULT FALSE,
			is_syncing			BOOL NOT NULL DEFAULT FALSE,
			last_synced			TIMESTAMPTZ DEFAULT NULL,

			CONSTRAINT ci_git_provider_ch_login_name_password_is_deleted
				CHECK(
					(
						is_deleted = TRUE AND
						password IS NULL
					) OR (
						is_deleted = FALSE AND (
							(
								password IS NOT NULL AND
								login_name IS NOT NULL
							) OR password IS NULL
						)
					)
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE UNIQUE INDEX
			ci_git_provider_uq_workspace_id_domain_name
		ON
			ci_git_provider(workspace_id, domain_name)
		WHERE
			is_deleted = FALSE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE CI_REPO_STATUS AS ENUM(
			'active',
			'inactive',
			'deleted'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE ci_repos(
			id 						UUID CONSTRAINT ci_repos_pk PRIMARY KEY,
			repo_owner 				TEXT NOT NULL,
			repo_name 				TEXT NOT NULL,
			clone_url 				TEXT NOT NULL,
			webhook_secret 			TEXT CONSTRAINT ci_repos_uq_secret UNIQUE,
			status 					CI_REPO_STATUS NOT NULL,
			git_provider_id 		UUID NOT NULL
				CONSTRAINT ci_repos_fk_git_provider_id
					REFERENCES ci_git_provider(id),
			git_provider_repo_uid 	TEXT NOT NULL,
			runner_id 				UUID
				CONSTRAINT ci_repos_fk_runner_id REFERENCES ci_runner(id),

			CONSTRAINT ci_repos_uq_git_provider_id_repo_uid
				UNIQUE(git_provider_id, git_provider_repo_uid),
			CONSTRAINT ci_repos_chk_status_runner_id_webhook_secret
				CHECK(
					(
						status = 'active' AND
						runner_id IS NOT NULL AND
						webhook_secret IS NOT NULL
					) OR status != 'active'
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE CI_BUILD_STATUS AS ENUM(
			'waiting_to_start',
			'running',
			'succeeded',
			'cancelled',
			'errored'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE ci_builds(
			repo_id 			UUID NOT NULL
				CONSTRAINT ci_builds_fk_repo_id REFERENCES ci_repos(id),
			build_num 			BIGINT NOT NULL
				CONSTRAINT ci_builds_chk_build_num_unsigned CHECK(build_num > 0),
			git_ref 			TEXT NOT NULL,
			git_commit 			TEXT NOT NULL,
			status 				CI_BUILD_STATUS NOT NULL,
			created 			TIMESTAMPTZ NOT NULL,
			finished 			TIMESTAMPTZ,
			message				TEXT,
			author				TEXT,
			git_commit_message	TEXT,
			git_pr_title		TEXT,
			started 			TIMESTAMPTZ,
			runner_id 			UUID NOT NULL
				CONSTRAINT ci_builds_fk_runner_id REFERENCES ci_runner(id),

			CONSTRAINT ci_builds_pk_repo_id_build_num
				PRIMARY KEY(repo_id, build_num)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE CI_BUILD_STEP_STATUS AS ENUM(
			'waiting_to_start',
			'running',
			'succeeded',
			'cancelled',
			'errored',
			'skipped_dep_error'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE ci_steps(
			repo_id 	UUID NOT NULL,
			build_num 	BIGINT NOT NULL,
			step_id 	INTEGER NOT NULL
				CONSTRAINT ci_steps_chk_step_id_unsigned CHECK(build_num >= 0),
			step_name 	TEXT NOT NULL,
			base_image 	TEXT NOT NULL,
			commands 	TEXT NOT NULL,
			status 		CI_BUILD_STEP_STATUS NOT NULL,
			started 	TIMESTAMPTZ,
			finished 	TIMESTAMPTZ,

			CONSTRAINT ci_steps_fk_repo_id_build_num
				FOREIGN KEY(repo_id, build_num)
					REFERENCES ci_builds(repo_id, build_num),
			CONSTRAINT ci_steps_pk_repo_id_build_num_step_id
				PRIMARY KEY(repo_id, build_num, step_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_ci_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Finishing up ci tables initialization");

	// machine types for ci builds
	const CI_BUILD_MACHINE_TYPES: [(i32, i32, i32); 5] = [
		(1, 2, 1),  // 1 vCPU, 0.5 GB RAM, 1GB storage
		(1, 4, 1),  // 1 vCPU,   1 GB RAM, 1GB storage
		(1, 6, 2),  // 1 vCPU,   2 GB RAM, 2GB storage
		(2, 8, 4),  // 2 vCPU,   4 GB RAM, 4GB storage
		(4, 32, 8), // 4 vCPU,   8 GB RAM, 8GB storage
	];

	for (cpu, ram, volume) in CI_BUILD_MACHINE_TYPES {
		let machine_type_id =
			db::generate_new_resource_id(&mut *connection).await?;
		query!(
			r#"
			INSERT INTO
				ci_build_machine_type(
					id,
					cpu,
					ram,
					volume
				)
			VALUES
				($1, $2, $3, $4);
			"#,
			machine_type_id as _,
			cpu,
			ram,
			volume
		)
		.execute(&mut *connection)
		.await?;
	}

	runner::initialize_ci_runner_constraints(connection).await?;

	Ok(())
}

pub async fn get_all_build_machine_types(
	connection: &mut DatabaseConnection,
) -> Result<Vec<BuildMachineType>, sqlx::Error> {
	query_as!(
		BuildMachineType,
		r#"
		SELECT
			id as "id: _",
			cpu,
			ram,
			volume
		FROM
			ci_build_machine_type;
		"#,
	)
	.fetch_all(connection)
	.await
}

pub async fn add_git_provider_to_workspace(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	git_provider_domain: &str,
	git_provider_type: GitProviderType,
	login_name: Option<&str>,
	password: Option<&str>,
) -> Result<Uuid, sqlx::Error> {
	let id = Uuid::new_v4();

	query!(
		r#"
		INSERT INTO
			ci_git_provider(
				id,
				workspace_id,
				domain_name,
				git_provider_type,
				login_name,
				password
			)
		VALUES
			($1, $2, $3, $4, $5, $6);
		"#,
		id as _,
		workspace_id as _,
		git_provider_domain,
		git_provider_type as _,
		login_name,
		password,
	)
	.execute(&mut *connection)
	.await
	.map(|_| id)
}

pub async fn get_git_provider_details_by_id(
	connection: &mut DatabaseConnection,
	git_provider_id: &Uuid,
) -> Result<Option<GitProvider>, sqlx::Error> {
	query_as!(
		GitProvider,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			domain_name,
			git_provider_type as "git_provider_type: _",
			login_name,
			password,
			is_syncing,
			last_synced,
			is_deleted
		FROM
			ci_git_provider
		WHERE
			id = $1;
		"#,
		git_provider_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn remove_git_provider_credentials(
	connection: &mut DatabaseConnection,
	git_provider_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query_as!(
		GitProvider,
		r#"
		UPDATE
			ci_git_provider
		SET
			is_deleted = TRUE,
			password = NULL
		WHERE
			id = $1;
		"#,
		git_provider_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn list_connected_git_providers_for_workspace(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
) -> Result<Vec<GitProvider>, sqlx::Error> {
	query_as!(
		GitProvider,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			domain_name,
			git_provider_type as "git_provider_type: _",
			login_name,
			password,
			is_syncing,
			last_synced,
			is_deleted
		FROM
			ci_git_provider
		WHERE
			workspace_id = $1 AND
			is_deleted = FALSE;
		"#,
		workspace_id as _,
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_git_provider_details_for_workspace_using_domain(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	domain_name: &str,
) -> Result<Option<GitProvider>, sqlx::Error> {
	query_as!(
		GitProvider,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			domain_name,
			git_provider_type as "git_provider_type: _",
			login_name,
			password,
			is_syncing,
			last_synced,
			is_deleted
		FROM
			ci_git_provider
		WHERE
			workspace_id = $1 AND
			domain_name = $2 AND
			is_deleted = FALSE;
		"#,
		workspace_id as _,
		domain_name,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn add_repo_for_git_provider(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	git_provider_id: &Uuid,
	git_provider_repo_uid: &str,
	repo_owner: &str,
	repo_name: &str,
	clone_url: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			ci_repos(
				id,
				repo_owner,
				repo_name,
				clone_url,
				webhook_secret,
				status,
				git_provider_id,
				git_provider_repo_uid,
				runner_id
			)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, $9);
		"#,
		repo_id as _,
		repo_owner,
		repo_name,
		clone_url,
		Option::<&str>::None,
		RepoStatus::Inactive as _,
		git_provider_id as _,
		git_provider_repo_uid,
		Option::<&Uuid>::None as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_repo_details_for_git_provider(
	connection: &mut DatabaseConnection,
	git_provider_id: &Uuid,
	git_provider_repo_uid: &str,
	repo_owner: &str,
	repo_name: &str,
	clone_url: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			ci_repos
		SET
			repo_owner = $3,
			repo_name = $4,
			clone_url = $5
		WHERE
			git_provider_id = $1 AND
			git_provider_repo_uid = $2;
		"#,
		git_provider_id as _,
		git_provider_repo_uid,
		repo_owner,
		repo_name,
		clone_url,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn list_repos_for_git_provider(
	connection: &mut DatabaseConnection,
	git_provider_id: &Uuid,
) -> Result<Vec<Repository>, sqlx::Error> {
	query_as!(
		Repository,
		r#"
		SELECT
			id as "id: _",
			repo_owner,
			repo_name,
			clone_url,
			webhook_secret,
			status as "status: _",
			git_provider_id as "git_provider_id: _",
			git_provider_repo_uid,
			runner_id as "runner_id: _"
		FROM
			ci_repos
		WHERE
			git_provider_id = $1;
		"#,
		git_provider_id as _,
	)
	.fetch_all(connection)
	.await
}

// since we currently only have github route only, using github direclty in the
// where clause while supporting multiple git providers, we need to pass git
// provider id
pub async fn get_repo_details_using_github_uid_for_workspace(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
	git_provider_repo_uid: &str,
) -> Result<Option<Repository>, sqlx::Error> {
	query_as!(
		Repository,
		r#"
		SELECT
			ci_repos.id as "id: _",
			repo_owner,
			repo_name,
			clone_url,
			webhook_secret,
			status as "status: _",
			git_provider_id as "git_provider_id: _",
			git_provider_repo_uid,
			runner_id as "runner_id: _"
		FROM
			ci_repos
		INNER JOIN
			ci_git_provider
		ON
			ci_git_provider.id = ci_repos.git_provider_id
		WHERE
			ci_git_provider.workspace_id = $1 AND
			ci_repos.git_provider_repo_uid = $2 AND
			ci_git_provider.domain_name = 'github.com' AND
			ci_git_provider.is_deleted = FALSE;
		"#,
		workspace_id as _,
		git_provider_repo_uid
	)
	.fetch_optional(connection)
	.await
}

pub async fn update_repo_status(
	connection: &mut DatabaseConnection,
	git_provider_id: &Uuid,
	git_provider_repo_uid: &str,
	status: RepoStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			ci_repos
		SET
			status = $3
		WHERE
			git_provider_id = $1 AND
			git_provider_repo_uid = $2;
		"#,
		git_provider_id as _,
		git_provider_repo_uid,
		status as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn activate_ci_for_repo(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	runner_id: &Uuid,
) -> Result<String, sqlx::Error> {
	let secret = Uuid::new_v4().to_string();

	query!(
		r#"
		UPDATE
			ci_repos
		SET
			runner_id = $2,
			webhook_secret = $3,
			status = 'active'
		WHERE
			id = $1;
		"#,
		repo_id as _,
		runner_id as _,
		secret
	)
	.execute(&mut *connection)
	.await
	.map(|_| secret)
}

pub async fn get_repo_using_patr_repo_id(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
) -> Result<Option<Repository>, sqlx::Error> {
	query_as!(
		Repository,
		r#"
		SELECT
			id as "id: _",
			repo_owner,
			repo_name,
			clone_url,
			webhook_secret,
			status as "status: _",
			git_provider_id as "git_provider_id: _",
			git_provider_repo_uid,
			runner_id as "runner_id: _"
		FROM
			ci_repos
		WHERE
			id = $1;
		"#,
		repo_id as _,
	)
	.fetch_optional(connection)
	.await
}

pub async fn generate_new_build_for_repo(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	git_ref: &str,
	git_commit: &str,
	author: Option<&str>,
	git_commit_message: Option<&str>,
	git_pr_title: Option<&str>,
	runner_id: &Uuid,
) -> Result<i64, sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			ci_builds(
				repo_id,
				build_num,
				git_ref,
				git_commit,
				status,
				author,
				git_commit_message,
				git_pr_title,
				runner_id,
				created
			)
		VALUES(
			$1,
			1 + (SELECT COUNT(*) FROM ci_builds WHERE repo_id = $1),
			$2,
			$3,
			$4,
			$5,
			$6,
			$7,
			$8,
			$9
		)
		RETURNING build_num;
		"#,
		repo_id as _,
		git_ref as _,
		git_commit as _,
		BuildStatus::WaitingToStart as _,
		author as _,
		git_commit_message as _,
		git_pr_title as _,
		runner_id as _,
		Utc::now()
	)
	.fetch_one(connection)
	.await
	.map(|row| row.build_num)
}

pub async fn update_build_status(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	build_num: i64,
	status: BuildStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			ci_builds
		SET
			status = $3
		WHERE
			repo_id = $1 AND
			build_num = $2;
		"#,
		repo_id as _,
		build_num,
		status as _
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn get_build_status(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	build_num: i64,
) -> Result<Option<BuildStatus>, sqlx::Error> {
	let result = query_as!(
		BuildRecord,
		r#"
		SELECT
			build_num,
			git_ref,
			git_commit,
			status as "status: _",
			created as "created: _",
			started as "started: _",
			finished as "finished: _",
			message,
			author,
			git_pr_title,
			git_commit_message,
			runner_id as "runner_id: _"
		FROM
			ci_builds
		WHERE
			repo_id = $1 AND
			build_num = $2;
		"#,
		repo_id as _,
		build_num,
	)
	.fetch_optional(connection)
	.await?
	.map(|row| row.status);

	Ok(result)
}

// https://shiroyasha.io/selecting-for-share-and-update-in-postgresql.html
// the selected row is locked until the transaction ends
pub async fn get_build_status_for_update(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	build_num: i64,
) -> Result<Option<BuildStatus>, sqlx::Error> {
	let result = query_as!(
		BuildRecord,
		r#"
		SELECT
			build_num,
			git_ref,
			git_commit,
			status as "status: _",
			created as "created: _",
			started as "started: _",
			finished as "finished: _",
			message,
			author,
			git_pr_title,
			git_commit_message,
			runner_id as "runner_id: _"
		FROM
			ci_builds
		WHERE
			repo_id = $1 AND
			build_num = $2
		FOR UPDATE;
		"#,
		repo_id as _,
		build_num,
	)
	.fetch_optional(connection)
	.await?
	.map(|row| row.status);

	Ok(result)
}

pub async fn update_build_started_time(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	build_num: i64,
	started: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			ci_builds
		SET
			started = $3
		WHERE
			repo_id = $1 AND
			build_num = $2;
		"#,
		repo_id as _,
		build_num,
		started as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_build_finished_time(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	build_num: i64,
	finished: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			ci_builds
		SET
			finished = $3
		WHERE
			repo_id = $1 AND
			build_num = $2;
		"#,
		repo_id as _,
		build_num,
		finished as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_build_message(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	build_num: i64,
	message: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			ci_builds
		SET
			message = $3
		WHERE
			repo_id = $1 AND
			build_num = $2;
		"#,
		repo_id as _,
		build_num,
		message
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn list_build_steps_for_build(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	build_num: i64,
) -> Result<Vec<Step>, sqlx::Error> {
	query_as!(
		StepRecord,
		r#"
		SELECT
			step_id,
			step_name,
			base_image,
			commands,
			status as "status: _",
			started as "started: _",
			finished as "finished: _"
		FROM
			ci_steps
		WHERE
			repo_id = $1 AND
			build_num = $2
		ORDER BY
			step_id ASC;
		"#,
		repo_id as _,
		build_num
	)
	.fetch(connection)
	.map_ok(|step_record| Step {
		step_id: step_record.step_id as u32,
		step_name: step_record.step_name,
		base_image: step_record.base_image,
		commands: step_record.commands,
		status: step_record.status,
		started: step_record.started.map(utils::DateTime),
		finished: step_record.finished.map(utils::DateTime),
	})
	.try_collect()
	.await
}

pub async fn list_build_details_for_repo(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
) -> Result<Vec<BuildDetails>, sqlx::Error> {
	query_as!(
		BuildRecord,
		r#"
		SELECT
			build_num,
			git_ref,
			git_commit,
			status as "status: _",
			created as "created: _",
			started as "started: _",
			finished as "finished: _",
			message,
			author,
			git_pr_title,
			git_commit_message,
			runner_id as "runner_id: _"
		FROM
			ci_builds
		WHERE
			repo_id = $1
		ORDER BY
			build_num DESC;
		"#,
		repo_id as _,
	)
	.fetch(&mut *connection)
	.map_ok(|build| BuildDetails {
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
		runner_id: build.runner_id,
	})
	.try_collect()
	.await
}

pub async fn get_build_details_for_build(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	build_num: i64,
) -> Result<Option<BuildDetails>, sqlx::Error> {
	query_as!(
		BuildRecord,
		r#"
		SELECT
			build_num,
			git_ref,
			git_commit,
			status as "status: _",
			created as "created: _",
			started as "started: _",
			finished as "finished: _",
			message,
			author,
			git_pr_title,
			git_commit_message,
			runner_id as "runner_id: _"
		FROM
			ci_builds
		WHERE
			repo_id = $1 AND
			build_num = $2
		ORDER BY
			build_num DESC;
		"#,
		repo_id as _,
		build_num
	)
	.fetch_optional(&mut *connection)
	.await
	.map(|optional_build| {
		optional_build.map(|build| BuildDetails {
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
			runner_id: build.runner_id,
		})
	})
}

pub async fn add_ci_step_for_build(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	build_num: i64,
	step_id: i32,
	step_name: &str,
	base_image: &str,
	commands: &str,
	status: BuildStepStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			ci_steps(
				repo_id,
				build_num,
				step_id,
				step_name,
				base_image,
				commands,
				status
			)
		VALUES
			($1, $2, $3, $4, $5, $6, $7);
		"#,
		repo_id as _,
		build_num,
		step_id,
		step_name,
		base_image,
		commands,
		status as _,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn update_build_step_status(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	build_num: i64,
	step_id: i32,
	status: BuildStepStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			ci_steps
		SET
			status = $4
		WHERE
			repo_id = $1 AND
			build_num = $2 AND
			step_id = $3;
		"#,
		repo_id as _,
		build_num,
		step_id,
		status as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_build_step_status(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	build_num: i64,
	step_id: i32,
) -> Result<Option<BuildStepStatus>, sqlx::Error> {
	query!(
		r#"
		SELECT
			status as "status: BuildStepStatus"
		FROM
			ci_steps
		WHERE
			repo_id = $1 AND
			build_num = $2 AND
			step_id = $3;
		"#,
		repo_id as _,
		build_num,
		step_id,
	)
	.fetch_optional(&mut *connection)
	.await
	.map(|row| row.map(|row| row.status))
}

pub async fn get_build_created_time(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	build_num: i64,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
	struct QueryResult {
		created: Option<DateTime<Utc>>,
	}

	let created_time = query_as!(
		QueryResult,
		r#"
		SELECT
			created as "created: _"
		FROM
			ci_builds
		WHERE
			repo_id = $1 AND
			build_num = $2;
		"#,
		repo_id as _,
		build_num,
	)
	.fetch_optional(&mut *connection)
	.await?
	.and_then(|row| row.created);

	Ok(created_time)
}

pub async fn update_build_step_started_time(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	build_num: i64,
	step_id: i32,
	started: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			ci_steps
		SET
			started = $4
		WHERE
			repo_id = $1 AND
			build_num = $2 AND
			step_id = $3;
		"#,
		repo_id as _,
		build_num,
		step_id,
		started as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_build_step_finished_time(
	connection: &mut DatabaseConnection,
	repo_id: &Uuid,
	build_num: i64,
	step_id: i32,
	finished: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			ci_steps
		SET
			finished = $4
		WHERE
			repo_id = $1 AND
			build_num = $2 AND
			step_id = $3;
		"#,
		repo_id as _,
		build_num,
		step_id,
		finished as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn set_syncing(
	connection: &mut DatabaseConnection,
	git_provider_id: &Uuid,
	is_syncing: bool,
	last_synced: Option<DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
	query_as!(
		GitProvider,
		r#"
		UPDATE
			ci_git_provider
		SET
			is_syncing = $1,
			last_synced = COALESCE($2, last_synced)
		WHERE
			id = $3;
		"#,
		is_syncing,
		last_synced as _,
		git_provider_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_recent_activity_for_ci_in_workspace(
	connection: &mut DatabaseConnection,
	workspace_id: &Uuid,
) -> Result<Vec<RecentActivity>, sqlx::Error> {
	query!(
		r#"
		SELECT
			ci_git_provider.id as "git_provider_id: Uuid",
			ci_git_provider.workspace_id as "git_provider_workspace_id: Uuid",
			ci_git_provider.domain_name as "git_provider_domain_name",
			ci_git_provider.git_provider_type as "git_provider_type: GitProviderType",
			ci_git_provider.login_name as "git_provider_login_name",
			ci_git_provider.is_syncing as "git_provider_is_syncing",
			ci_git_provider.last_synced as "git_provider_last_synced",
			ci_git_provider.is_deleted as "git_provider_is_deleted",
			ci_repos.git_provider_repo_uid as "repo_id",
			ci_repos.repo_name,
			ci_repos.repo_owner,
			ci_repos.clone_url as "repo_clone_url",
			ci_repos.status as "repo_status: RepoStatus",
			ci_repos.runner_id as "repo_runner_id: Uuid",
			ci_builds.build_num,
			ci_builds.git_ref as "build_git_ref",
			ci_builds.git_commit as "build_git_commit",
			ci_builds.status as "build_status: BuildStatus",
			ci_builds.created as "build_created",
			ci_builds.started as "build_started",
			ci_builds.finished as "build_finished",
			ci_builds.message as "build_message",
			ci_builds.author as "build_author",
			ci_builds.git_pr_title as "build_git_pr_title",
			ci_builds.git_commit_message as "build_git_commit_message",
			ci_builds.runner_id as "build_runner_id: Uuid"
		FROM
			ci_builds
		JOIN
			ci_repos
		ON
			ci_repos.id = ci_builds.repo_id
		JOIN
			ci_git_provider 
		ON
			ci_git_provider.id = ci_repos.git_provider_id
		WHERE
			ci_git_provider.workspace_id = $1
		ORDER BY
			ci_builds.created ASC
		LIMIT 500;
		"#,
		workspace_id as _,
	)
	.fetch(&mut *connection)
	.map_ok(|activity| RecentActivity {
		git_provider_details:
			api_models::models::workspace::ci::git_provider::GitProvider {
				id: activity.git_provider_id,
				domain_name: activity.git_provider_domain_name,
				git_provider_type: activity.git_provider_type,
				login_name: activity.git_provider_login_name,
				is_syncing: activity.git_provider_is_syncing,
				last_synced: activity
					.git_provider_last_synced
					.map(api_models::utils::DateTime),
				is_deleted: activity.git_provider_is_deleted,
			},
		repo_details: RepositoryDetails {
			id: activity.repo_id,
			name: activity.repo_name,
			repo_owner: activity.repo_owner,
			clone_url: activity.repo_clone_url,
			status: activity.repo_status,
			runner_id: activity.repo_runner_id,
		},
		build_details: BuildDetails {
			build_num: activity.build_num as u64,
			git_ref: activity.build_git_ref,
			git_commit: activity.build_git_commit,
			git_commit_message: activity.build_git_commit_message,
			git_pr_title: activity.build_git_pr_title,
			author: activity.build_author,
			status: activity.build_status,
			created: api_models::utils::DateTime(activity.build_created),
			started: activity.build_started.map(api_models::utils::DateTime),
			finished: activity.build_finished.map(api_models::utils::DateTime),
			message: activity.build_message,
			runner_id: activity.build_runner_id,
		},
	})
	.try_collect()
	.await
}
