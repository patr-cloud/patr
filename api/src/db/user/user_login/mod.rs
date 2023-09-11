mod api_token;
mod web_login;

use api_macros::query;
use api_models::utils::Uuid;
use chrono::{DateTime, Utc};

pub use self::{api_token::*, web_login::*};
use crate::Database;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "USER_LOGIN_TYPE", rename_all = "snake_case")]
pub enum UserLoginType {
	ApiToken,
	WebLogin,
}

pub async fn initialize_user_login_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TYPE USER_LOGIN_TYPE AS ENUM(
			'api_token',
			'web_login'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_login(
			login_id UUID CONSTRAINT user_login_pk PRIMARY KEY,
			user_id UUID NOT NULL
				CONSTRAINT user_login_fk_user_id REFERENCES "user"(id),
            login_type USER_LOGIN_TYPE NOT NULL,
			created TIMESTAMPTZ NOT NULL,
			CONSTRAINT user_login_uq_login_id_user_id UNIQUE(login_id, user_id),
			CONSTRAINT user_login_uq_login_id_user_id_login_type UNIQUE(
				login_id, user_id, login_type
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	web_login::initialize_web_login_pre(&mut *connection).await?;
	api_token::initialize_api_token_pre(&mut *connection).await?;

	Ok(())
}

pub async fn initialize_user_login_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	web_login::initialize_web_login_post(&mut *connection).await?;
	api_token::initialize_api_token_post(&mut *connection).await?;

	Ok(())
}

pub async fn generate_new_login_id(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<Uuid, sqlx::Error> {
	loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				login_id
			FROM
				user_login
			WHERE
				login_id = $1;
			"#,
			uuid as _
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break Ok(uuid);
		}
	}
}

pub async fn add_new_user_login(
	connection: &mut <Database as sqlx::Database>::Connection,
	login_id: &Uuid,
	user_id: &Uuid,
	login_type: &UserLoginType,
	created: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			user_login(
				login_id,
				user_id,
				login_type,
				created
			)
		VALUES
			($1, $2, $3, $4);
		"#,
		login_id as _,
		user_id as _,
		login_type as _,
		created as _,
	)
	.execute(connection)
	.await
	.map(|_| ())
}
