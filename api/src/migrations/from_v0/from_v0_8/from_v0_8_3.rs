use api_models::utils::Uuid;
use chrono::{Datelike, TimeZone, Utc};
use sqlx::Row;

use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

#[derive(sqlx::Type, PartialEq, Eq, Hash)]
#[sqlx(type_name = "TRANSACTION_TYPE", rename_all = "lowercase")]
pub enum TransactionType {
	Bill,
	Credits,
	Payment,
}

#[derive(sqlx::Type, PartialEq, Eq, Hash)]
#[sqlx(type_name = "PAYMENT_STATUS", rename_all = "lowercase")]
pub enum PaymentStatus {
	Pending,
	Success,
	Failed,
}

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		DROP TABLE workspace_credits;
		"#
	)
	.execute(&mut *connection)
	.await?;

	let workspaces = query!(
		r#"
		SELECT
			workspace.id as "id",
			resource.created as "created"
		FROM
			workspace
		INNER JOIN
			resource
		ON
			workspace.id = resource.id;
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| (row.get::<Uuid, _>("id"), row.get::<i64, _>("created")));

	for (workspace_id, created) in workspaces {
		let created = Utc.timestamp_millis_opt(created).unwrap();
		let transaction_id = loop {
			let uuid = Uuid::new_v4();

			let exists = query!(
				r#"
				SELECT
					id
				FROM
					transaction
				WHERE
					id = $1;
				"#,
				&uuid
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
			INSERT INTO
				transaction(
					id,
					month,
					amount,
					payment_intent_id,
					date,
					workspace_id,
					transaction_type,
					payment_status,
					description
				)
				VALUES
				(
					$1,
					$2,
					$3,
					$4,
					$5,
					$6,
					$7,
					$8,
					$9
				);
			"#,
			&transaction_id,
			created.month() as i32,
			25f64,
			Some("sign-up-credits"),
			&created,
			&workspace_id,
			&TransactionType::Credits,
			&PaymentStatus::Success,
			Some("Sign up credits"),
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}
