use std::net::IpAddr;

use api_models::utils::Uuid;
use chrono::{DateTime, Utc};

use crate::{query, query_as, Database};

pub struct UserWebLogin {
	pub login_id: Uuid,
	pub user_id: Uuid,

	/// Hashed refresh token
	pub refresh_token: String,
	pub token_expiry: DateTime<Utc>,

	pub created: DateTime<Utc>,
	pub created_ip: IpAddr,
	pub created_location_latitude: f64,
	pub created_location_longitude: f64,
	pub created_country: String,
	pub created_region: String,
	pub created_city: String,
	pub created_timezone: String,

	pub last_login: DateTime<Utc>,
	pub last_activity: DateTime<Utc>,
	pub last_activity_ip: IpAddr,
	pub last_activity_location_latitude: f64,
	pub last_activity_location_longitude: f64,
	pub last_activity_country: String,
	pub last_activity_region: String,
	pub last_activity_city: String,
	pub last_activity_timezone: String,
	pub last_activity_user_agent: String,
}

pub async fn initialize_web_login_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TABLE web_login(
			login_id UUID NOT NULL,
			user_id UUID NOT NULL,

			refresh_token TEXT NOT NULL,
			token_expiry TIMESTAMPTZ NOT NULL,

			created TIMESTAMPTZ NOT NULL,
			created_ip INET NOT NULL,
			created_location GEOMETRY NOT NULL,
			created_country TEXT NOT NULL,
			created_region TEXT NOT NULL,
			created_city TEXT NOT NULL,
			created_timezone TEXT NOT NULL,

			last_login TIMESTAMPTZ NOT NULL,
			last_activity TIMESTAMPTZ NOT NULL,
			last_activity_ip INET NOT NULL,
			last_activity_location GEOMETRY NOT NULL,
			last_activity_country TEXT NOT NULL,
			last_activity_region TEXT NOT NULL,
			last_activity_city TEXT NOT NULL,
			last_activity_timezone TEXT NOT NULL,
			last_activity_user_agent TEXT NOT NULL,

			login_type USER_LOGIN_TYPE NOT NULL
				GENERATED ALWAYS AS ('web_login') STORED,
			CONSTRAINT web_login_fk FOREIGN KEY(login_id, user_id, login_type)
				REFERENCES user_login(login_id, user_id, login_type)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			web_login_idx_user_id
		ON
			web_login(user_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE INDEX
			web_login_idx_login_id
		ON
			web_login(login_id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_web_login_post(
	_connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn add_new_web_login(
	connection: &mut <Database as sqlx::Database>::Connection,
	login_id: &Uuid,
	user_id: &Uuid,

	refresh_token: &str,
	token_expiry: &DateTime<Utc>,

	created: &DateTime<Utc>,
	created_ip: &IpAddr,
	created_location_latitude: f64,
	created_location_longitude: f64,
	created_country: &str,
	created_region: &str,
	created_city: &str,
	created_timezone: &str,

	last_login: &DateTime<Utc>,
	last_activity: &DateTime<Utc>,
	last_activity_ip: &IpAddr,
	last_activity_location_latitude: f64,
	last_activity_location_longitude: f64,
	last_activity_country: &str,
	last_activity_region: &str,
	last_activity_city: &str,
	last_activity_timezone: &str,
	last_activity_user_agent: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		INSERT INTO
			web_login(
				login_id,
				user_id, 

				refresh_token, 
				token_expiry, 

				created,
				created_ip,
				created_location,
				created_country,
				created_region,
				created_city,
				created_timezone,

				last_login, 
				last_activity,
				last_activity_ip,
				last_activity_location,
				last_activity_country,
				last_activity_region,
				last_activity_city,
				last_activity_timezone,
				last_activity_user_agent
			)
		VALUES
			(
				$1,
				$2,

				$3,
				$4,

				$5,
				$6,
				ST_SetSRID(POINT($7, $8)::GEOMETRY, 4326),
				$9,
				$10,
				$11,
				$12,

				$13,
				$14,
				$15,
				ST_SetSRID(POINT($16, $17)::GEOMETRY, 4326),
				$18,
				$19,
				$20,
				$21,
				$22
			);
		"#,
		login_id as _,
		user_id as _,
		refresh_token,
		token_expiry as _,
		created as _,
		created_ip as _,
		created_location_latitude as _,
		created_location_longitude as _,
		created_country,
		created_region,
		created_city,
		created_timezone,
		last_login as _,
		last_activity as _,
		last_activity_ip as _,
		last_activity_location_latitude as _,
		last_activity_location_longitude as _,
		last_activity_country,
		last_activity_region,
		last_activity_city,
		last_activity_timezone,
		last_activity_user_agent,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_user_web_login(
	connection: &mut <Database as sqlx::Database>::Connection,
	login_id: &Uuid,
) -> Result<Option<UserWebLogin>, sqlx::Error> {
	query_as!(
		UserWebLogin,
		r#"
		SELECT
			login_id as "login_id: _",
			refresh_token,
			token_expiry,
			user_id as "user_id: _",
			created,
			created_ip as "created_ip: _",
			ST_X(ST_Centroid(ST_Transform(created_location, 4326))) as "created_location_latitude!: _",
			ST_Y(ST_Centroid(ST_Transform(created_location, 4326))) as "created_location_longitude!: _",
			created_country,
			created_region,
			created_city,
			created_timezone,
			last_login,
			last_activity,
			last_activity_ip as "last_activity_ip: _",
			ST_X(ST_Centroid(ST_Transform(last_activity_location, 4326))) as "last_activity_location_latitude!: _",
			ST_Y(ST_Centroid(ST_Transform(last_activity_location, 4326))) as "last_activity_location_longitude!: _",
			last_activity_country,
			last_activity_region,
			last_activity_city,
			last_activity_timezone,
			last_activity_user_agent
		FROM
			web_login
		WHERE
			login_id = $1;
		"#,
		login_id as _
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn get_user_web_login_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	login_id: &Uuid,
	user_id: &Uuid,
) -> Result<Option<UserWebLogin>, sqlx::Error> {
	query_as!(
		UserWebLogin,
		r#"
		SELECT
			login_id as "login_id: _",
			refresh_token,
			token_expiry,
			user_id as "user_id: _",
			created,
			created_ip as "created_ip: _",
			ST_X(ST_Centroid(ST_Transform(created_location, 4326))) as "created_location_latitude!: _",
			ST_Y(ST_Centroid(ST_Transform(created_location, 4326))) as "created_location_longitude!: _",
			created_country,
			created_region,
			created_city,
			created_timezone,
			last_login,
			last_activity,
			last_activity_ip as "last_activity_ip: _",
			ST_X(ST_Centroid(ST_Transform(last_activity_location, 4326))) as "last_activity_location_latitude!: _",
			ST_Y(ST_Centroid(ST_Transform(last_activity_location, 4326))) as "last_activity_location_longitude!: _",
			last_activity_country,
			last_activity_region,
			last_activity_city,
			last_activity_timezone,
			last_activity_user_agent
		FROM
			web_login
		WHERE
			login_id = $1 AND
			user_id = $2;
		"#,
		login_id as _,
		user_id as _,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn update_user_web_login_last_activity_info(
	connection: &mut <Database as sqlx::Database>::Connection,
	login_id: &Uuid,
	last_activity: &DateTime<Utc>,
	last_activity_ip: &IpAddr,
	last_activity_location_latitude: f64,
	last_activity_location_longitude: f64,
	last_activity_country: &str,
	last_activity_region: &str,
	last_activity_city: &str,
	last_activity_timezone: &str,
	last_activity_user_agent: &str,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			web_login
		SET
			last_activity = $1,
			last_activity_ip = $2,
			last_activity_location = ST_SetSRID(POINT($3, $4)::GEOMETRY, 4326),
			last_activity_country = $5,
			last_activity_region = $6,
			last_activity_city = $7,
			last_activity_timezone = $8,
			last_activity_user_agent = $9
		WHERE
			login_id = $10;
		"#,
		last_activity,
		last_activity_ip as _,
		last_activity_location_latitude,
		last_activity_location_longitude,
		last_activity_country,
		last_activity_region,
		last_activity_city,
		last_activity_timezone,
		last_activity_user_agent,
		login_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn get_all_web_logins_for_user(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
) -> Result<Vec<UserWebLogin>, sqlx::Error> {
	query_as!(
		UserWebLogin,
		r#"
		SELECT
			login_id as "login_id: _",
			refresh_token,
			token_expiry,
			user_id as "user_id: _",
			created as "created: _",
			created_ip as "created_ip: _",
			ST_X(ST_Centroid(ST_Transform(created_location, 4326))) as "created_location_latitude!: _",
			ST_Y(ST_Centroid(ST_Transform(created_location, 4326))) as "created_location_longitude!: _",
			created_country,
			created_region,
			created_city,
			created_timezone,
			last_login,
			last_activity,
			last_activity_ip as "last_activity_ip: _",
			ST_X(ST_Centroid(ST_Transform(last_activity_location, 4326))) as "last_activity_location_latitude!: _",
			ST_Y(ST_Centroid(ST_Transform(last_activity_location, 4326))) as "last_activity_location_longitude!: _",
			last_activity_country,
			last_activity_region,
			last_activity_city,
			last_activity_timezone,
			last_activity_user_agent
		FROM
			web_login
		WHERE
			user_id = $1;
		"#,
		user_id as _
	)
	.fetch_all(&mut *connection)
	.await
}

pub async fn get_web_login_for_user_with_refresh_token(
	connection: &mut <Database as sqlx::Database>::Connection,
	user_id: &Uuid,
	refresh_token: &str,
) -> Result<Option<UserWebLogin>, sqlx::Error> {
	query_as!(
		UserWebLogin,
		r#"
		SELECT
			login_id as "login_id: _",
			refresh_token,
			token_expiry,
			user_id as "user_id: _",
			created as "created: _",
			created_ip as "created_ip: _",
			ST_X(ST_Centroid(ST_Transform(created_location, 4326))) as "created_location_latitude!: _",
			ST_Y(ST_Centroid(ST_Transform(created_location, 4326))) as "created_location_longitude!: _",
			created_country,
			created_region,
			created_city,
			created_timezone,
			last_login,
			last_activity,
			last_activity_ip as "last_activity_ip: _",
			ST_X(ST_Centroid(ST_Transform(last_activity_location, 4326))) as "last_activity_location_latitude!: _",
			ST_Y(ST_Centroid(ST_Transform(last_activity_location, 4326))) as "last_activity_location_longitude!: _",
			last_activity_country,
			last_activity_region,
			last_activity_city,
			last_activity_timezone,
			last_activity_user_agent
		FROM
			web_login
		WHERE
			user_id = $1 AND
			refresh_token = $2;
		"#,
		user_id as _,
		refresh_token,
	)
	.fetch_optional(&mut *connection)
	.await
}

pub async fn delete_user_web_login_by_id(
	connection: &mut <Database as sqlx::Database>::Connection,
	login_id: &Uuid,
	user_id: &Uuid,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			web_login
		SET
			token_expiry = TO_TIMESTAMP(0)
		WHERE
			login_id = $1 AND
			user_id = $2;
		"#,
		login_id as _,
		user_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}

pub async fn set_web_login_expiry(
	connection: &mut <Database as sqlx::Database>::Connection,
	login_id: &Uuid,
	last_activity: &DateTime<Utc>,
	token_expiry: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		UPDATE
			web_login
		SET
			token_expiry = $1,
			last_activity = $2
		WHERE
			login_id = $3;
		"#,
		token_expiry as _,
		last_activity as _,
		login_id as _,
	)
	.execute(&mut *connection)
	.await
	.map(|_| ())
}
