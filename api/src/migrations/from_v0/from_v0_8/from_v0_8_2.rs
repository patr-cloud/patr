use api_models::utils::Uuid;
use serde::Deserialize;
use sqlx::Row;

use crate::{
	migrate_query as query,
	utils::{settings::Settings, Error},
	Database,
};

pub(super) async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TYPE STATIC_SITE_PLAN AS ENUM(
			'free',
			'pro',
			'unlimited'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DOMAIN_PLAN AS ENUM(
			'free',
			'unlimited'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS deployment_payment_history(
			workspace_id UUID NOT NULL,
			deployment_id UUID NOT NULL,
			machine_type UUID NOT NULL,
			num_instance INTEGER NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS static_sites_payment_history(
			workspace_id UUID NOT NULL,
			static_site_plan STATIC_SITE_PLAN NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS managed_database_payment_history(
			workspace_id UUID NOT NULL,
			database_id UUID NOT NULL,
			db_plan MANAGED_DATABASE_PLAN NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			deletion_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS managed_url_payment_history(
			workspace_id UUID NOT NULL,
			url_count INTEGER NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS secrets_payment_history(
			workspace_id UUID NOT NULL,
			secret_count INTEGER NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS docker_repo_payment_history(
			workspace_id UUID NOT NULL,
			storage BIGINT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS domain_payment_history(
			workspace_id UUID NOT NULL,
			domain_plan DOMAIN_PLAN NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS payment_method(
			payment_method_id TEXT CONSTRAINT payment_method_pk PRIMARY KEY,
			workspace_id UUID NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE TRANSACTION_TYPE as ENUM(
			'bill',
			'credits',
			'payment'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE PAYMENT_STATUS as ENUM(
			'success',
			'failed'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS transaction(
			id UUID CONSTRAINT transaction_pk PRIMARY KEY,
			month INTEGER NOT NULL,
			amount DOUBLE PRECISION NOT NULL,
			payment_intent_id TEXT,
			date TIMESTAMPTZ NOT NULL,
			workspace_id UUID NOT NULL,
			transaction_type TRANSACTION_TYPE NOT NULL,
			payment_status PAYMENT_STATUS NOT NULL
				CONSTRAINT transaction_payment_status_check CHECK (
					(
					payment_status = 'success' AND
					transaction_type = 'bill'
					) OR
					(
						transaction_type != 'bill'
					)
				),
			description TEXT
				CONSTRAINT transaction_description_check CHECK (
					transaction_type = 'credits' AND
					description IS NOT NULL
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_payment_history
		ADD CONSTRAINT deployment_payment_history_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE static_sites_payment_history
		ADD CONSTRAINT static_sites_payment_history_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_database_payment_history
		ADD CONSTRAINT managed_database_payment_history_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE managed_url_payment_history
		ADD CONSTRAINT managed_url_payment_history_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE secrets_payment_history
		ADD CONSTRAINT secrets_payment_history_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE docker_repo_payment_history
		ADD CONSTRAINT docker_repo_payment_history_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE domain_payment_history
		ADD CONSTRAINT domain_payment_history_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE payment_method
		ADD CONSTRAINT payment_method_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE transaction
		ADD CONSTRAINT transaction_workspace_id_fk
		FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE PAYMENT_TYPE AS ENUM(
			'card',
			'enterprise'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
			ADD COLUMN
				payment_type PAYMENT_TYPE NOT NULL
				DEFAULT 'card',
			ADD COLUMN
				default_payment_method_id TEXT,
			ADD COLUMN
				deployment_limit INTEGER NOT NULL
				DEFAULT 10,
			ADD COLUMN
				database_limit INTEGER NOT NULL
				DEFAULT 5,
			ADD COLUMN
				static_site_limit INTEGER NOT NULL
				DEFAULT 30,
			ADD COLUMN
				managed_url_limit INTEGER NOT NULL
				DEFAULT 200,
			ADD COLUMN
				docker_repository_storage_limit INTEGER NOT NULL
				DEFAULT 200,
			ADD COLUMN
				domain_limit INTEGER NOT NULL
				DEFAULT 5,
			ADD COLUMN
				secret_limit INTEGER NOT NULL
				DEFAULT 150,
			ADD COLUMN
				stripe_customer_id TEXT,
			ADD COLUMN
				address_id UUID,
			ADD COLUMN
				amount_due DOUBLE PRECISION NOT NULL
				DEFAULT 0;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE address(
			id UUID NOT NULL CONSTRAINT address_pk PRIMARY KEY,
			first_name TEXT NOT NULL,
			last_name TEXT NOT NULL,
			address_line_1 TEXT NOT NULL,
			address_line_2 TEXT,
			address_line_3 TEXT,
			city TEXT NOT NULL,
			state TEXT NOT NULL,
			zip TEXT NOT NULL,
			country TEXT NOT NULL
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		ADD CONSTRAINT workspace_default_payment_method_id_fk
		FOREIGN KEY (default_payment_method_id)
			REFERENCES payment_method(payment_method_id)
		DEFERRABLE INITIALLY DEFERRED;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		ADD CONSTRAINT workspace_fk_address_id
		FOREIGN KEY(address_id) REFERENCES address(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	let workspaces = query!(
		r#"
		SELECT
			*
		FROM
			workspace;
		"#
	)
	.fetch_all(&mut *connection)
	.await?
	.into_iter()
	.map(|row| row.get::<Uuid, _>("id"));

	#[derive(Debug, Deserialize)]
	#[serde(rename_all = "camelCase")]
	pub struct StripeCustomer {
		pub id: String,
		pub name: String,
	}

	let client = reqwest::Client::new();
	for workspace_id in workspaces {
		let customer = client
			.post("https://api.stripe.com/v1/customers")
			.basic_auth(&config.stripe.secret_key, Option::<String>::None)
			.query(&[("name", workspace_id.as_str())])
			.send()
			.await?
			.json::<StripeCustomer>()
			.await?;
		query!(
			r#"
			UPDATE
				workspace
			SET
				stripe_customer_id = $1
			WHERE
				id = $2;
			"#,
			customer.id.as_str(),
			workspace_id,
		)
		.execute(&mut *connection)
		.await?;
	}

	// Remove all default column values that were set above
	query!(
		r#"
		ALTER TABLE workspace
			ALTER COLUMN payment_type DROP DEFAULT,
			ALTER COLUMN deployment_limit DROP DEFAULT,
			ALTER COLUMN database_limit DROP DEFAULT,
			ALTER COLUMN static_site_limit DROP DEFAULT,
			ALTER COLUMN managed_url_limit DROP DEFAULT,
			ALTER COLUMN docker_repository_storage_limit DROP DEFAULT,
			ALTER COLUMN domain_limit DROP DEFAULT,
			ALTER COLUMN secret_limit DROP DEFAULT,
			ALTER COLUMN stripe_customer_id SET NOT NULL,
			ALTER COLUMN amount_due DROP DEFAULT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		ADD COLUMN workspace_limit INTEGER NOT NULL
		DEFAULT 3;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE "user"
		ALTER COLUMN workspace_limit
		DROP DEFAULT;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
