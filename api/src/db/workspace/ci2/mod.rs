use api_models::{
	models::workspace::ci2::{
		github::{Build, BuildStatus, BuildStepStatus, Step},
		BuildMachineType,
	},
	utils::Uuid,
};
use chrono::{DateTime, Utc};

use crate::{db, query, query_as, Database};

pub struct Repository {
	pub id: Uuid,
	pub workspace_id: Uuid,
	pub repo_owner: String,
	pub repo_name: String,
	pub git_url: String,
	pub webhook_secret: String,
	pub build_machine_type_id: Uuid,
	pub active: bool,
}

struct BuildTableRecord {
	pub repo_id: Uuid,
	pub build_num: i64,
	pub git_ref: String,
	pub git_commit: String,
	pub status: BuildStatus,
	pub created: DateTime<Utc>,
	pub finished: Option<DateTime<Utc>>,
}

pub async fn initialize_ci_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing ci tables");

	query!(
		r#"
		CREATE TABLE ci_build_machine_type (
			id UUID CONSTRAINT ci_machint_type_pk PRIMARY KEY,
			cpu INTEGER NOT NULL CONSTRAINT ci_build_machine_type_chk_cpu_positive CHECK (cpu > 0),   		/* Multiples of 1 vCPU */
			ram INTEGER NOT NULL CONSTRAINT ci_build_machine_type_chk_ram_positive CHECK (ram > 0),   		/* Multiples of 0.25 GB RAM */
			volume INTEGER NOT NULL CONSTRAINT ci_build_machine_type_chk_volume_positive CHECK (volume > 0) 	/* Multiples of 1 GB storage space */
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE ci_repos (
			id UUID CONSTRAINT ci_repos_pk PRIMARY KEY,
			workspace_id UUID NOT NULL CONSTRAINT ci_repos_fk_workspace_id REFERENCES workspace(id),
			repo_owner TEXT NOT NULL,
			repo_name TEXT NOT NULL,
			git_url TEXT NOT NULL,
			webhook_secret TEXT NOT NULL CONSTRAINT ci_repos_uq_secret UNIQUE,
			build_machine_type_id UUID NOT NULL CONSTRAINT ci_repos_fk_build_machine_type_id REFERENCES ci_build_machine_type(id),
			active BOOLEAN NOT NULL,

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
			'stopped',
			'errored'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE ci_builds (
			repo_id UUID NOT NULL CONSTRAINT ci_builds_fk_repo_id REFERENCES ci_repos(id),
			build_num BIGINT NOT NULL CONSTRAINT ci_builds_chk_build_num_unsigned CHECK (build_num > 0),
			git_ref TEXT NOT NULL,
			git_commit TEXT NOT NULL,
			status CI_BUILD_STATUS NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			finished TIMESTAMPTZ,

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
			'stopped',
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
			repo_id UUID NOT NULL,
			build_num BIGINT NOT NULL,
			step_id INTEGER NOT NULL CONSTRAINT ci_steps_chk_step_id_unsigned CHECK (build_num >= 0),
			step_name TEXT NOT NULL,
			base_image TEXT NOT NULL,
			commands TEXT[] NOT NULL,
			status CI_BUILD_STEP_STATUS NOT NULL,
			started TIMESTAMPTZ,
			finished TIMESTAMPTZ,

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
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up ci tables initialization");

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

	Ok(())
}

pub async fn get_all_build_machine_types(
	connection: &mut <Database as sqlx::Database>::Connection,
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
			ci_build_machine_type
		"#,
	)
	.fetch_all(connection)
	.await
}

pub async fn create_ci_repo(
	connection: &mut <Database as sqlx::Database>::Connection,
	workspace_id: &Uuid,
	repo_owner: &str,
	repo_name: &str,
	git_url: &str,
	build_machine_type_id: &Uuid,
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
			build_machine_type_id,
			active
		)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, FALSE)
		"#,
		repo_id as _,
		workspace_id as _,
		repo_owner,
		repo_name as _,
		git_url as _,
		webhook_secret as _,
		build_machine_type_id as _,
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
		build_machine_type_id: build_machine_type_id.to_owned(),
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
			build_machine_type_id as "build_machine_type_id: _",
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
			build_machine_type_id as "build_machine_type_id: _",
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
			build_machine_type_id as "build_machine_type_id: _",
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

pub async fn update_build_machine_type_for_repo(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
	build_machine_type_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			ci_repos
		SET
			build_machine_type_id = $2
		WHERE
			id = $1;
		"#,
		repo_id as _,
		build_machine_type_id as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_build_machine_type_for_repo(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
) -> Result<Option<BuildMachineType>, sqlx::Error> {
	query_as!(
		BuildMachineType,
		r#"
		SELECT
			ci_build_machine_type.id as "id: _",
			cpu,
			ram,
			volume
		FROM
			ci_build_machine_type
		JOIN ci_repos
			ON ci_repos.build_machine_type_id = ci_build_machine_type.id
		WHERE
			ci_repos.id = $1;
		"#,
		repo_id as _,
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
			build_machine_type_id as "build_machine_type_id: _",
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

pub async fn get_build_status(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
	build_num: i64,
) -> Result<Option<BuildStatus>, sqlx::Error> {
	let result = query_as!(
		BuildTableRecord,
		r#"
		SELECT
			repo_id as "repo_id: _",
			build_num,
			git_ref,
			git_commit,
			status as "status: _",
			created,
			finished
		FROM
			ci_builds
		WHERE
			(repo_id = $1 AND build_num = $2);
		"#,
		repo_id as _,
		build_num,
	)
	.fetch_optional(connection)
	.await?
	.map(|row| row.status);

	Ok(result)
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

pub async fn list_build_steps_for_build(
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
			ORDER BY step_id ASC;
		"#,
		repo_id as _,
		build_num
	)
	.fetch_all(connection)
	.await
}

pub async fn list_build_details_for_repo(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
) -> Result<Vec<Build>, sqlx::Error> {
	let builds = query_as!(
		BuildTableRecord,
		r#"
		SELECT
			repo_id as "repo_id: _",
			build_num,
			git_ref,
			git_commit,
			status as "status: _",
			created,
			finished
		FROM
			ci_builds
		WHERE
			repo_id = $1
		ORDER BY build_num DESC;
		"#,
		repo_id as _,
	)
	.fetch_all(&mut *connection)
	.await?;

	let mut result = vec![];
	for build in builds {
		let steps = list_build_steps_for_build(
			&mut *connection,
			repo_id,
			build.build_num,
		)
		.await?;
		result.push(Build {
			repo_id: build.repo_id,
			build_num: build.build_num,
			git_ref: build.git_ref,
			git_commit: build.git_commit,
			status: build.status,
			created: build.created,
			finished: build.finished,
			steps,
		})
	}

	Ok(result)
}

pub async fn get_build_details_for_build(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
	build_num: i64,
) -> Result<Option<Build>, sqlx::Error> {
	let build = query_as!(
		BuildTableRecord,
		r#"
		SELECT
			repo_id as "repo_id: _",
			build_num,
			git_ref,
			git_commit,
			status as "status: _",
			created,
			finished
		FROM
			ci_builds
		WHERE (
			repo_id = $1
			AND build_num = $2
		)
		ORDER BY build_num DESC;
		"#,
		repo_id as _,
		build_num
	)
	.fetch_optional(&mut *connection)
	.await?;

	let result = match build {
		Some(build) => Some(Build {
			repo_id: build.repo_id,
			build_num: build.build_num,
			git_ref: build.git_ref,
			git_commit: build.git_commit,
			status: build.status,
			created: build.created,
			finished: build.finished,
			steps: list_build_steps_for_build(
				&mut *connection,
				repo_id,
				build.build_num,
			)
			.await?,
		}),
		None => None,
	};

	Ok(result)
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
