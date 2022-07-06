use api_models::{
	models::workspace::ci2::github::{Build, EnvVariable, Step},
	utils::Uuid,
};
use sqlx::postgres::PgTypeInfo;

use crate::{query, query_as, Database};

pub struct Repository {
	pub id: Uuid,
	pub workspace_id: Uuid,
	pub git_url: String,
	pub webhook_secret: String,
	pub active: bool,
}

// https://github.com/launchbadge/sqlx/pull/1170#issuecomment-817738085
#[derive(sqlx::Encode)]
struct EnvVariables<'a>(&'a [EnvVariable]);

impl sqlx::Type<sqlx::Postgres> for EnvVariables<'_> {
	fn type_info() -> PgTypeInfo {
		PgTypeInfo::with_name("_env_variable")
	}
}

pub async fn initialize_ci_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing ci tables");

	query!(
		r#"
		CREATE TABLE ci_repos (
            id UUID 
                CONSTRAINT ci_repos_pk PRIMARY KEY,
            workspace_id UUID NOT NULL,
			repo_name TEXT NOT NULL,
            git_url TEXT NOT NULL,
            webhook_secret TEXT NOT NULL 
                CONSTRAINT ci_repos_uq_secret UNIQUE,
            active BOOLEAN NOT NULL,
        
            CONSTRAINT ci_repos_fk_workspace_id 
                FOREIGN KEY (workspace_id) 
                    REFERENCES workspace(id),
            CONSTRAINT ci_repos_uq_workspace_id_git_url 
                UNIQUE (workspace_id, git_url) 
        );
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE ci_builds (
			repo_id UUID NOT NULL
				CONSTRAINT ci_builds_fk_repo_id
					REFERENCES ci_repos(id),
			build_num BIGINT NOT NULL
				CONSTRAINT ci_builds_chk_build_num_unsigned
					CHECK (build_num > 0),
			git_ref TEXT NOT NULL,
			git_commit TEXT NOT NULL,
			build_status TEXT,
			build_started TIMESTAMPTZ,
			build_finished TIMESTAMPTZ,

			CONSTRAINT ci_builds_pk_repo_id_build_num
				PRIMARY KEY (repo_id, build_num)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE env_variable AS (
			name    text,
			value   text
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
			step_id INTEGER NOT NULL,
			step_name TEXT NOT NULL,
			base_image TEXT NOT NULL,
			commands TEXT[] NOT NULL,
			env env_variable[] NOT NULL,
			step_status TEXT,

			CONSTRAINT ci_steps_fk_repo_id_build_num
				FOREIGN KEY (repo_id, build_num)
					REFERENCES ci_builds(repo_id, build_num),
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
	repo_name: &str,
	git_url: &str,
) -> Result<Repository, sqlx::Error> {
	let webhook_secret = loop {
		let uuid = Uuid::new_v4().to_string();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
                ci_repos
			WHERE
                webhook_secret = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break uuid;
		}
	};

	let repo_id = loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
                ci_repos
			WHERE
                id = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break uuid;
		}
	};

	query!(
		r#"
		INSERT INTO ci_repos (
            id,
            workspace_id,
			repo_name,
            git_url,
            webhook_secret,
            active
        )
		VALUES
			($1, $2, $3, $4, $5, FALSE)
		"#,
		repo_id as _,
		workspace_id as _,
		repo_name as _,
		git_url as _,
		webhook_secret as _,
	)
	.execute(&mut *connection)
	.await?;

	Ok(Repository {
		id: repo_id,
		workspace_id: workspace_id.to_owned(),
		git_url: git_url.to_owned(),
		webhook_secret: webhook_secret.to_owned(),
		active: false,
	})
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
			git_url::TEXT as "git_url!: _",
			webhook_secret::TEXT as "webhook_secret!: _",
			active as "active: _"
		FROM
			ci_repos
		WHERE (
            workspace_id = $1 
                AND git_url = $2 
        );
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
        UPDATE ci_repos
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
        UPDATE ci_repos
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
			git_url::TEXT as "git_url!: _",
			webhook_secret::TEXT as "webhook_secret!: _",
			active as "active: _"
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
) -> Result<i64, sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			ci_builds (repo_id, build_num, git_ref, git_commit)
		VALUES (
			$1,
			1 + (SELECT COUNT(*) FROM ci_builds WHERE repo_id = $1),
			$2,
			$3
		)
		RETURNING build_num;
		"#,
		repo_id as _,
		git_ref as _,
		git_commit as _
	)
	.fetch_one(connection)
	.await
	.map(|row| row.build_num)
}

pub async fn list_build_details_for_repo(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
) -> Result<Vec<Build>, sqlx::Error> {
	let builds = query!(
		r#"
 		SELECT
 			build_num,
 			git_ref,
 			git_commit
		FROM
 			ci_builds
 		WHERE
 			repo_id = $1
 		ORDER BY 
			build_num DESC
 		LIMIT 10;
 		"#,
		repo_id as _,
	)
	.fetch_all(&mut *connection)
	.await?;

	let mut result = Vec::new();
	for build in builds {
		let steps = get_build_steps_for_build(
			&mut *connection,
			repo_id,
			build.build_num,
		)
		.await?;

		result.extend(std::iter::once(Build {
			repo_id: repo_id.clone(),
			build_num: build.build_num,
			git_ref: build.git_ref,
			git_commit: build.git_commit,
			steps,
		}));
	}

	Ok(result)
}

pub async fn get_build_details_for_build(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
	build_num: i64,
) -> Result<Build, sqlx::Error> {
	let build = query!(
		r#"
 		SELECT
 			build_num,
 			git_ref,
 			git_commit
		FROM
 			ci_builds
 		WHERE (
			repo_id = $1
			AND build_num = $2
		)
		ORDER BY 
			build_num DESC
 		LIMIT 10;
 		"#,
		repo_id as _,
		build_num
	)
	.fetch_one(&mut *connection)
	.await?;

	let steps =
		get_build_steps_for_build(&mut *connection, repo_id, build_num).await?;

	Ok(Build {
		repo_id: repo_id.clone(),
		build_num,
		git_ref: build.git_ref,
		git_commit: build.git_commit,
		steps,
	})
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
			commands
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

// 	query_as!(
// 		Build,
// 		r#"
// 		SELECT
// 			ci_builds.build_num,
// 			ci_builds.git_ref,
// 			ci_builds.git_commit,
// 			ARRAY_AGG (
// 				(
// 					ci_steps.step_id,
// 					ci_steps.step_name,
// 					ci_steps.base_image,
// 					ci_steps.commands,
// 					ci_steps.env
// 				)
// 				ORDER BY ci_steps.step_id ASC
// 			) as "steps!: Vec<Step>"
// 		FROM
// 			ci_builds
// 		INNER JOIN
// 			ci_steps
// 				ON (
// 					ci_builds.repo_id = ci_steps.repo_id
// 					AND ci_builds.build_num = ci_steps.build_num
// 				)
// 		WHERE
// 			ci_builds.repo_id = $1
// 		GROUP BY (
// 			ci_builds.build_num,
// 			ci_builds.git_ref,
// 			ci_builds.git_commit
// 		)
// 		ORDER BY ci_builds.build_num DESC
// 		LIMIT 10;
// 		"#,
// 		repo_id as _,
// 	)
// 	.fetch_all(connection)
// 	.await

pub async fn add_ci_steps_for_build(
	connection: &mut <Database as sqlx::Database>::Connection,
	repo_id: &Uuid,
	build_num: i64,
	step_id: u32,
	step_name: &str,
	base_image: &str,
	commands: Vec<String>,
	env: Vec<EnvVariable>,
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
			env
		)
		VALUES
			($1, $2, $3, $4, $5, $6, $7);
		"#,
		repo_id as _,
		build_num,
		step_id as _,
		step_name as _,
		base_image as _,
		&commands[..],
		EnvVariables(&env) as _
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
