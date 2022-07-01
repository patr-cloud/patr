use api_models::utils::Uuid;

use crate::{query, query_as, Database};

pub struct Repository {
	pub id: Uuid,
	pub workspace_id: Uuid,
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
            id UUID 
                CONSTRAINT ci_repos_pk PRIMARY KEY,
            workspace_id UUID NOT NULL,
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
			id BIGINT NOT NULL 
				CONSTRAINT ci_builds_chk_id_unsigned 
					CHECK (id > 0),
			repo_id UUID NOT NULL,
		
			CONSTRAINT ci_builds_fk_repo_id 
				FOREIGN KEY (repo_id) 
					REFERENCES ci_repos(id),
			CONSTRAINT ci_builds_pk_id_repo_id 
				PRIMARY KEY (id, repo_id)
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
            git_url,
            webhook_secret,
            active
        )
		VALUES
			($1, $2, $3, $4, FALSE)
		"#,
		repo_id as _,
		workspace_id as _,
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
) -> Result<i64, sqlx::Error> {
	query!(
		r#"
		INSERT INTO 
			ci_builds (id, repo_id) 
		VALUES (
			1 + (SELECT COUNT(*) FROM ci_builds WHERE repo_id = $1),
			$1
		)
		RETURNING id;
		"#,
		repo_id as _
	)
	.fetch_one(connection)
	.await
	.map(|row| row.id)
}
