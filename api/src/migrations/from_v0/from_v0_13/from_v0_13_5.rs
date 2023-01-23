use api_models::utils::Uuid;

use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	reset_invalid_birthday(&mut *connection, config).await?;

	// Volumes migration
	add_volume_resource_type(&mut *connection, config).await?;
	add_volume_payment_history(&mut *connection, config).await?;
	add_deployment_volume_info(&mut *connection, config).await?;
	add_volume_storage_limit_to_workspace(&mut *connection, config).await?;

	Ok(())
}

pub(super) async fn reset_invalid_birthday(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	log::info!("Updating all invalid user dob");

	query!(
		r#"
		UPDATE
			"user"
		SET
			dob = NULL
		WHERE
			dob IS NOT NULL AND
			dob > (NOW() - INTERVAL '13 YEARS');
		"#,
	)
	.execute(&mut *connection)
	.await?;

	log::info!("All invalid dobs updated");

	query!(
		r#"
		ALTER TABLE "user" 
		ADD CONSTRAINT user_chk_dob_is_13_plus 
		CHECK (dob IS NULL OR dob < (NOW() - INTERVAL '13 YEARS'));             
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_volume_resource_type(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	let resource_type_id = loop {
		let resource_type_id = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				resource_type
			WHERE
				id = $1;
			"#,
			&resource_type_id
		)
		.fetch_optional(&mut *connection)
		.await?
		.is_some();

		if !exists {
			break resource_type_id;
		}
	};

	query!(
		r#"
		INSERT INTO
			resource_type(
				id,
				name,
				description
			)
		VALUES
			($1, 'volume', '');
		"#,
		&resource_type_id
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_volume_payment_history(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	// transaction table migrations
	query!(
		r#"
		CREATE TABLE volume_payment_history(
			workspace_id UUID NOT NULL,
			volume_id UUID NOT NULL,
			storage BIGINT NOT NULL,
			number_of_volumes INTEGER NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_deployment_volume_info(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TABLE deployment_volume(
			id UUID CONSTRAINT deployment_volume_pk PRIMARY KEY,
			deployment_id UUID NOT NULL
				CONSTRAINT deployment_volume_fk_deployment_id
					REFERENCES deployment(id),
			volume_size INT NOT NULL CONSTRAINT
				deployment_volume_chk_size_unsigned
					CHECK(volume_size > 0),
			volume_mount_path TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn add_volume_storage_limit_to_workspace(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace
		ADD COLUMN volume_storage_limit INTEGER NOT NULL DEFAULT 100;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		ALTER COLUMN volume_storage_limit DROP DEFAULT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
