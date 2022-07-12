use api_models::{
	models::workspace::ci2::github::{
		Build,
		BuildStatus,
		BuildStepStatus,
		Step,
	},
	utils::Uuid,
};
use chrono::{DateTime, Utc};

use crate::{query, query_as, Database};

pub struct Repository {
	pub id: Uuid,
	pub workspace_id: Uuid,
	pub repo_owner: String,
	pub repo_name: String,
	pub git_url: String,
	pub webhook_secret: String,
	pub active: bool,
}

pub async fn initialize_ci_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing ci tables");

	query!(
		r#"
		CREATE TABLE ci_repos (
            id 				UUID CONSTRAINT ci_repos_pk PRIMARY KEY,
            workspace_id 	UUID NOT NULL,
			repo_owner 		TEXT NOT NULL,
			repo_name 		TEXT NOT NULL,
            git_url 		TEXT NOT NULL,
            webhook_secret 	TEXT NOT NULL CONSTRAINT ci_repos_uq_secret UNIQUE,
            active 			BOOLEAN NOT NULL,

            CONSTRAINT ci_repos_fk_workspace_id
				FOREIGN KEY (workspace_id) REFERENCES workspace(id),
            CONSTRAINT ci_repos_uq_workspace_id_git_url
				UNIQUE (workspace_id, git_url),
			CONSTRAINT ci_repos_uq_workspace_id_repo_owner_repo_name
				UNIQUE (workspace_id, repo_owner, repo_name)
        );
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE CI_BUILD_STATUS AS ENUM (
			'running',
			'succeeded',
			'errored'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE ci_builds (
			repo_id		UUID NOT NULL CONSTRAINT ci_builds_fk_repo_id REFERENCES ci_repos(id),
			build_num 	BIGINT NOT NULL CONSTRAINT ci_builds_chk_build_num_unsigned CHECK (build_num > 0),
			git_ref 	TEXT NOT NULL,
			git_commit 	TEXT NOT NULL,
			status 		CI_BUILD_STATUS NOT NULL,
			created 	TIMESTAMPTZ NOT NULL,
			finished 	TIMESTAMPTZ,

			CONSTRAINT ci_builds_pk_repo_id_build_num
				PRIMARY KEY (repo_id, build_num)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE CI_BUILD_STEP_STATUS AS ENUM (
			'waiting_to_start',
			'running',
			'succeeded',
			'errored',
			'skipped_dep_error'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE ci_steps (
			repo_id 	UUID NOT NULL,
			build_num 	BIGINT NOT NULL,
			step_id 	INTEGER NOT NULL CONSTRAINT ci_steps_chk_step_id_unsigned CHECK (build_num >= 0),
			step_name 	TEXT NOT NULL,
			base_image 	TEXT NOT NULL,
			commands 	TEXT[] NOT NULL,
			status 		CI_BUILD_STEP_STATUS NOT NULL,
			started 	TIMESTAMPTZ,
			finished 	TIMESTAMPTZ,

			CONSTRAINT ci_steps_fk_repo_id_build_num
				FOREIGN KEY (repo_id, build_num) REFERENCES ci_builds(repo_id, build_num),
			CONSTRAINT ci_steps_pk_repo_id_build_num_step_id
				PRIMARY KEY (repo_id, build_num, step_id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_ci_post(
	_connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up ci tables initialization");

	Ok(())
}

pub async fn create_ci_repo(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	repo_owner: &str,
	repo_name: &str,
	git_url: &str,
) -> Result<Repository, sqlx::Error> {
	let webhook_secret = Uuid::new_v4().to_string();
	let repo_id = Uuid::new_v4();

	query!(
		r#"
		INSERT INTO ci_repos (
            id,
            workspace_id,
			repo_owner,
			repo_name,
            git_url,
            webhook_secret,
            active
        )
		VALUES
			($1, $2, $3, $4, $5, $6, FALSE)
		"#,
		repo_id as _,
		workspace_id as _,
		repo_owner,
		repo_name as _,
		git_url as _,
		webhook_secret as _,
	)
	.execute(&mut *connection)
	.await?;

	Ok(Repository {
		id: repo_id,
		workspace_id: workspace_id.to_owned(),
		repo_owner: repo_owner.to_owned(),
		repo_name: repo_name.to_owned(),
		git_url: git_url.to_owned(),
		webhook_secret: webhook_secret.to_owned(),
		active: false,
	})
}

pub async fn get_all_repos_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
) -> Result<Vec<Repository>, sqlx::Error> {
	query_as!(
		Repository,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			repo_owner,
			repo_name,
			git_url,
			webhook_secret,
			active
		FROM
			ci_repos
		WHERE
			workspace_id = $1;
		"#,
		workspace_id as _,
	)
	.fetch_all(connection)
	.await
}

pub async fn get_repo_for_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	repo_owner: &str,
	repo_name: &str,
) -> Result<Option<Repository>, sqlx::Error> {
	query_as!(
		Repository,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			repo_owner,
			repo_name,
			git_url,
			webhook_secret,
			active
		FROM
			ci_repos
		WHERE (
			workspace_id = $1
			AND repo_owner = $2
			AND repo_name = $3
		);
		"#,
		workspace_id as _,
		repo_owner,
		repo_name,
	)
	.fetch_optional(connection)
	.await
}

pub async fn get_repo_for_workspace_and_url(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	git_url: &str,
) -> Result<Option<Repository>, sqlx::Error> {
	query_as!(
		Repository,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			repo_owner,
			repo_name,
			git_url,
			webhook_secret,
			active
		FROM
			ci_repos
		WHERE
			(workspace_id = $1 AND git_url = $2);
		"#,
		workspace_id as _,
		git_url as _
	)
	.fetch_optional(connection)
	.await
}

pub async fn activate_ci_for_repo(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
        UPDATE
			ci_repos
        SET
            active = TRUE
        WHERE
            id = $1;
		"#,
		repo_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn deactivate_ci_for_repo(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
        UPDATE
			ci_repos
        SET
            active = FALSE
        WHERE
            id = $1;
		"#,
		repo_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_repo_for_git_url(
	connection: &mut <Database as sqlx::Database>::Connection,
	git_url: &str,
) -> Result<Vec<Repository>, sqlx::Error> {
	query_as!(
		Repository,
		r#"
		SELECT
			id as "id: _",
			workspace_id as "workspace_id: _",
			repo_owner,
			repo_name,
			git_url,
			webhook_secret,
			active
		FROM
			ci_repos
		WHERE
            git_url = $1;
		"#,
		git_url as _
	)
	.fetch_all(connection)
	.await
}

pub async fn get_access_token_for_repo(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
) -> Result<Option<String>, sqlx::Error> {
	query!(
		r#"
		SELECT
			drone_token
		FROM
			workspace
		JOIN ci_repos
			ON ci_repos.workspace_id = workspace.id
		WHERE
			ci_repos.id = $1;
		"#,
		repo_id as _
	)
	.fetch_one(connection)
	.await
	.map(|row| row.drone_token)
}

pub async fn generate_new_build_for_repo(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
	git_ref: &str,
	git_commit: &str,
	status: BuildStatus,
	created: &DateTime<Utc>,
) -> Result<i64, sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			ci_builds (repo_id, build_num, git_ref, git_commit, status, created)
		VALUES (
			$1,
			1 + (SELECT COUNT(*) FROM ci_builds WHERE repo_id = $1),
			$2,
			$3,
			$4,
			$5
		)
		RETURNING build_num;
		"#,
		repo_id as _,
		git_ref as _,
		git_commit as _,
		status as _,
		created,
	)
	.fetch_one(connection)
	.await
	.map(|row| row.build_num)
}

pub async fn update_build_status(
	connection: &mut <Database as sqlx::Database>::Connection,
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
			(repo_id = $1 AND build_num = $2);
		"#,
		repo_id as _,
		build_num,
		status as _
	)
	.execute(connection)
	.await
	.map(|_| ())
}

pub async fn update_build_finished_time(
	connection: &mut <Database as sqlx::Database>::Connection,
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
		WHERE (
			repo_id = $1
			AND build_num = $2
		);
		"#,
		repo_id as _,
		build_num,
		finished
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn list_build_details_for_repo(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
) -> Result<Vec<Build>, sqlx::Error> {
	query_as!(
		Build,
		r#"
		SELECT
			ci_builds.repo_id as "repo_id: _",
			ci_builds.build_num,
			ci_builds.git_ref,
			ci_builds.git_commit,
			ci_builds.status as "status: _",
			ci_builds.created,
			ci_builds.finished,
			ARRAY_AGG (
				(
					ci_steps.step_id,
					ci_steps.step_name,
					ci_steps.base_image,
					ci_steps.commands,
					ci_steps.status,
					ci_steps.started,
					ci_steps.finished
				)
				ORDER BY ci_steps.step_id ASC
			) as "steps!: Vec<Step>"
		FROM
			ci_builds
		INNER JOIN
			ci_steps
				ON (
					ci_builds.repo_id = ci_steps.repo_id
					AND ci_builds.build_num = ci_steps.build_num
				)
		WHERE
			ci_builds.repo_id = $1
		GROUP BY (
			ci_builds.repo_id,
			ci_builds.build_num
		)
		ORDER BY ci_builds.build_num DESC;
		"#,
		repo_id as _,
	)
	.fetch_all(connection)
	.await
}

pub async fn get_build_details_for_build(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
	build_num: i64,
) -> Result<Option<Build>, sqlx::Error> {
	query_as!(
		Build,
		r#"
		SELECT
			ci_builds.repo_id as "repo_id: _",
			ci_builds.build_num,
			ci_builds.git_ref,
			ci_builds.git_commit,
			ci_builds.status as "status: _",
			ci_builds.created,
			ci_builds.finished,
			ARRAY_AGG (
				(
					ci_steps.step_id,
					ci_steps.step_name,
					ci_steps.base_image,
					ci_steps.commands,
					ci_steps.status,
					ci_steps.started,
					ci_steps.finished
				)
				ORDER BY ci_steps.step_id ASC
			) as "steps!: Vec<Step>"
		FROM
			ci_builds
		INNER JOIN
			ci_steps
				ON (
					ci_builds.repo_id = ci_steps.repo_id
					AND ci_builds.build_num = ci_steps.build_num
				)
		WHERE (
			ci_builds.repo_id = $1
			AND ci_builds.build_num = $2
		)
		GROUP BY (
			ci_builds.repo_id,
			ci_builds.build_num
		)
		ORDER BY ci_builds.build_num DESC;
		"#,
		repo_id as _,
		build_num
	)
	.fetch_optional(connection)
	.await
}

pub async fn get_build_steps_for_build(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
	build_num: i64,
) -> Result<Vec<Step>, sqlx::Error> {
	query_as!(
		Step,
		r#"
		SELECT
			step_id,
			step_name,
			base_image,
			commands,
			status as "status: _",
			started,
			finished
		FROM
			ci_steps
		WHERE (
			repo_id = $1
			AND build_num = $2
		)
		ORDER BY
			step_id ASC;
		"#,
		repo_id as _,
		build_num
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn add_ci_steps_for_build(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
	build_num: i64,
	step_id: i32,
	step_name: &str,
	base_image: &str,
	commands: Vec<String>,
	status: BuildStepStatus,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO ci_steps (
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
		&commands[..],
		status as _,
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn update_build_step_status(
	connection: &mut <Database as sqlx::Database>::Connection,
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
		WHERE (
			repo_id = $1
			AND build_num = $2
			AND step_id = $3
		);
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
	connection: &mut <Database as sqlx::Database>::Connection,
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
		WHERE (
			repo_id = $1
			AND build_num = $2
			AND step_id = $3
		);
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
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
	build_num: i64,
) -> Result<Option<DateTime<Utc>>, sqlx::Error> {
	query!(
		r#"
		SELECT
			created
		FROM
			ci_builds
		WHERE (
			repo_id = $1
			AND build_num = $2
		);
		"#,
		repo_id as _,
		build_num,
	)
	.fetch_optional(&mut *connection)
	.await
	.map(|row| row.map(|row| row.created))
}

pub async fn update_build_step_started_time(
	connection: &mut <Database as sqlx::Database>::Connection,
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
		WHERE (
			repo_id = $1
			AND build_num = $2
			AND step_id = $3
		);
		"#,
		repo_id as _,
		build_num,
		step_id,
		started
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn update_build_step_finished_time(
	connection: &mut <Database as sqlx::Database>::Connection,
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
		WHERE (
			repo_id = $1
			AND build_num = $2
			AND step_id = $3
		);
		"#,
		repo_id as _,
		build_num,
		step_id,
		finished
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
