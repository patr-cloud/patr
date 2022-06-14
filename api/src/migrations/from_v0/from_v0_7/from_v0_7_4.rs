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
	add_github_permissions(&mut *connection, config).await?;
	add_alert_emails(&mut *connection, config).await?;
	update_workspace_with_ci_columns(&mut *connection, config).await?;
	reset_permission_order(&mut *connection, config).await?;

	// Adding Billing tables
	add_billing_tables(&mut *connection, config).await?;
	Ok(())
}

async fn add_github_permissions(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	for &permission in [
		"workspace::ci::github::connect",
		"workspace::ci::github::activate",
		"workspace::ci::github::deactivate",
		"workspace::ci::github::viewBuilds",
		"workspace::ci::github::restartBuilds",
		"workspace::ci::github::disconnect",
	]
	.iter()
	{
		let uuid = loop {
			let uuid = Uuid::new_v4();

			let exists = query!(
				r#"
				SELECT
					*
				FROM
					permission
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
				permission
			VALUES
				($1, $2, '');
			"#,
			&uuid,
			permission
		)
		.fetch_optional(&mut *connection)
		.await?;
	}

	Ok(())
}

async fn update_workspace_with_ci_columns(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace
			ADD COLUMN drone_username TEXT
				CONSTRAINT workspace_uq_drone_username UNIQUE,
			ADD COLUMN drone_token TEXT
				CONSTRAINT workspace_chk_drone_token_is_not_null
					CHECK(
						(
							drone_username IS NULL AND
							drone_token IS NULL
						) OR (
							drone_username IS NOT NULL AND
							drone_token IS NOT NULL
						)
					);
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

async fn reset_permission_order(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	for permission in [
		// Domain permissions
		"workspace::domain::list",
		"workspace::domain::add",
		"workspace::domain::viewDetails",
		"workspace::domain::verify",
		"workspace::domain::delete",
		// Dns record permissions
		"workspace::domain::dnsRecord::list",
		"workspace::domain::dnsRecord::add",
		"workspace::domain::dnsRecord::edit",
		"workspace::domain::dnsRecord::delete",
		// Deployment permissions
		"workspace::infrastructure::deployment::list",
		"workspace::infrastructure::deployment::create",
		"workspace::infrastructure::deployment::info",
		"workspace::infrastructure::deployment::delete",
		"workspace::infrastructure::deployment::edit",
		// Upgrade path permissions
		"workspace::infrastructure::upgradePath::list",
		"workspace::infrastructure::upgradePath::create",
		"workspace::infrastructure::upgradePath::info",
		"workspace::infrastructure::upgradePath::delete",
		"workspace::infrastructure::upgradePath::edit",
		// Managed URL permissions
		"workspace::infrastructure::managedUrl::list",
		"workspace::infrastructure::managedUrl::create",
		"workspace::infrastructure::managedUrl::edit",
		"workspace::infrastructure::managedUrl::delete",
		// Managed database permissions
		"workspace::infrastructure::managedDatabase::create",
		"workspace::infrastructure::managedDatabase::list",
		"workspace::infrastructure::managedDatabase::delete",
		"workspace::infrastructure::managedDatabase::info",
		// Static site permissions
		"workspace::infrastructure::staticSite::list",
		"workspace::infrastructure::staticSite::create",
		"workspace::infrastructure::staticSite::info",
		"workspace::infrastructure::staticSite::delete",
		"workspace::infrastructure::staticSite::edit",
		// Docker registry permissions
		"workspace::dockerRegistry::create",
		"workspace::dockerRegistry::list",
		"workspace::dockerRegistry::delete",
		"workspace::dockerRegistry::info",
		"workspace::dockerRegistry::push",
		"workspace::dockerRegistry::pull",
		// Secret permissions
		"workspace::secret::list",
		"workspace::secret::create",
		"workspace::secret::edit",
		"workspace::secret::delete",
		// RBAC Role permissions
		"workspace::rbac::role::list",
		"workspace::rbac::role::create",
		"workspace::rbac::role::edit",
		"workspace::rbac::role::delete",
		// RBAC User permissions
		"workspace::rbac::user::list",
		"workspace::rbac::user::add",
		"workspace::rbac::user::remove",
		"workspace::rbac::user::updateRoles",
		// CI permissions
		"workspace::ci::github::connect",
		"workspace::ci::github::activate",
		"workspace::ci::github::deactivate",
		"workspace::ci::github::viewBuilds",
		"workspace::ci::github::restartBuilds",
		"workspace::ci::github::disconnect",
		// Workspace permissions
		"workspace::edit",
		"workspace::delete",
	] {
		query!(
			r#"
			UPDATE
				permission
			SET
				name = CONCAT('test::', name)
			WHERE
				name = $1;
			"#,
			permission,
		)
		.execute(&mut *connection)
		.await?;

		query!(
			r#"
			UPDATE
				permission
			SET
				name = $1
			WHERE
				name = CONCAT('test::', $1);
			"#,
			&permission,
		)
		.execute(&mut *connection)
		.await?;
	}

	Ok(())
}

async fn add_alert_emails(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		ALTER TABLE workspace
			ADD COLUMN alert_emails VARCHAR(320) [] NOT NULL 
			DEFAULT ARRAY[]::VARCHAR[];
		"#,
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		ALTER TABLE workspace
			ALTER COLUMN alert_emails DROP DEFAULT;
		"#,
	)
	.execute(&mut *connection)
	.await?;
	query!(
		r#"
		UPDATE workspace w1
		SET alert_emails = (
			SELECT 
				ARRAY_AGG(CONCAT("user".recovery_email_local, '@', domain.name, '.', domain.tld))
			FROM 
				workspace w2
			INNER JOIN
				"user"
			ON
				"user".id = w2.super_admin_id
			INNER JOIN
				domain
			ON
				"user".recovery_email_domain_id = domain.id
			WHERE
				w2.id = w1.id
		);
		"#,
	)
	.execute(&mut *connection)
	.await?;
	Ok(())
}

async fn add_billing_tables(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), Error> {
	query!(
		r#"
		CREATE TYPE PLAN_TYPE AS ENUM(
			'one_time',
			'fixed_monthly',
			'dynamic_monthly'
			/* 'fixed_annually', */
			/* 'dynamic_annually', */
			/* 'fixed_resources', */
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE TRANSACTION_TYPE AS ENUM(
			'credits',
			'one_time',
			'fixed_monthly',
			'dynamic_monthly'
			/* 'fixed_annually', */
			/* 'dynamic_annually', */
			/* 'fixed_resources', */
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE PAYMENT_METHOD_TYPE AS ENUM(
			'credit_card',
			'debit_card'
	);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS product_info(
			id UUID CONSTRAINT product_info_pk PRIMARY KEY,
			name TEXT NOT NULL,
			description TEXT
		);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS plans(
			id UUID CONSTRAINT plans_pk PRIMARY KEY,
			name TEXT NOT NULL,
			description TEXT,
			plan_type PLAN_TYPE NOT NULL,
			product_info_id UUID NOT NULL,
			price DECIMAL(15, 6) NOT NULL,
			quantity INTEGER,
			workspace_id UUID NOT NULL,
			CONSTRAINT plans_product_info_id_fk 
			FOREIGN KEY (product_info_id) REFERENCES product_info(id),
			CONSTRAINT plans_workspace_id_fk
			FOREIGN KEY (workspace_id) REFERENCES workspace(id)
		);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS billable_service(
			id UUID CONSTRAINT billable_service_pk PRIMARY KEY,
			plan_id UUId NOT NULL,
			workspace_id UUID NOT NULL,
			price DECIMAL(15, 6) NOT NULL,
			quantity INTEGER NOT NULL,
			product_info_id UUID NOT NULL,
			total_price DECIMAL(15, 6) NOT NULL GENERATED ALWAYS AS (price * quantity) STORED,
			resource_id UUID NOT NULL,
			date TIMESTAMPTZ NOT NULL,
			active BOOLEAN NOT NULL,
			CONSTRAINT billable_service_plan_id_fk
			FOREIGN KEY (plan_id) REFERENCES plans(id),
			CONSTRAINT billable_service_workspace_id_fk
			FOREIGN KEY (workspace_id) REFERENCES workspace(id),
			CONSTRAINT billable_service_product_info_id_fk
			FOREIGN KEY (product_info_id) REFERENCES product_info(id),
			CONSTRAINT billable_service_resource_id_fk
			FOREIGN KEY (resource_id) REFERENCES resource(id)
		);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS coupons(
			id UUID CONSTRAINT coupons_pk PRIMARY KEY,
			name TEXT NOT NULL UNIQUE,
			description TEXT,
			credits DECIMAL(10, 2) NOT NULL,
			valid_from TIMESTAMPTZ NOT NULL,
			valid_till TIMESTAMPTZ NOT NULL,
			num_usage INTEGER NOT NULL	
		);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS transactions(
			id UUID CONSTRAINT transactions_pk PRIMARY KEY,
			billable_service_id UUID,
			product_info_id UUID NOT NULL,
			transaction_type TRANSACTION_TYPE NOT NULL,
			plan_id UUID,
			amount DECIMAL(15, 6) NOT NULL,
			quantity INTEGER,
			workspace_id UUID NOT NULL,
			date TIMESTAMPTZ NOT NULL,
			coupon_id UUID,
			debit bool NOT NULL,
			CONSTRAINT transactions_billable_service_id_fk
			FOREIGN KEY (billable_service_id) REFERENCES billable_service(id),
			CONSTRAINT transactions_product_info_id_fk
			FOREIGN KEY (product_info_id) REFERENCES product_info(id),
			CONSTRAINT transactions_plan_id_fk
			FOREIGN KEY (plan_id) REFERENCES plans(id),
			CONSTRAINT transactions_workspace_id_fk
			FOREIGN KEY (workspace_id) REFERENCES workspace(id),
			CONSTRAINT transactions_coupon_id_fk
			FOREIGN KEY (coupon_id) REFERENCES coupons(id)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS resource_limits(
			id UUID CONSTRAINT resource_limits_pk PRIMARY KEY,
			workspace_id UUID NOT NULL,
			max_resources INTEGER NOT NULL,
			CONSTRAINT resource_limits_workspace_id_fk
			FOREIGN KEY (workspace_id) REFERENCES workspace(id)
			DEFERRABLE INITIALLY IMMEDIATE;
		);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS product_limits(
			id UUID CONSTRAINT resource_usage_pk PRIMARY KEY,
			product_info_id UUID NOT NULL,
			max_limit INTEGER NOT NULL,
			workspace_id UUID NOT NULL,
			CONSTRAINT resource_usage_product_id_fk
			FOREIGN KEY (product_info_id) REFERENCES product_info(id),
			CONSTRAINT resource_usage_workspace_id_fk
			FOREIGN KEY (workspace_id) REFERENCES workspace(id);
		);
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE IF NOT EXISTS payment_method(
			id TEXT CONSTRAINT payment_method_pk PRIMARY KEY,
			method_type PAYMENT_METHOD_TYPE NOT NULL,
			workspace_id UUID NOT NULL,
			address_id UUID NOT NULL,
			CONSTRAINT payment_method_workspace_id_fk
			FOREIGN KEY (workspace_id) REFERENCES workspace(id),
			CONSTRAINT payment_method_address_id_fk
			FOREIGN KEY (address_id) REFERENCES workspace(address_id);
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		ADD COLUMN address_id UUID 
		CONSTRAINT workspace_fk_address_id REFERENCES address(id),
		ADD COLUMN resource_limits_id UUID 
		CONSTRAINT workspace_fk_resource_limit_id 
		REFERENCES resource_limits(id)
		DEFERRABLE INITIALLY IMMEDIATE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE MATERIALIZED VIEW credits
		AS
			SELECT SUM(
					CASE debit
					WHEN 'true' THEN 
						-amount
					ELSE 
						amount
					END
				) AS credits
			FROM transactions
			WHERE 
				billable_service_id IS NULL AND
				product_info_id IS NULL AND
				plan_id IS NULL
			GROUP BY workspace_id
		WITH NO DATA;
		/*This clause specifies whether or not the materialized view should be populated at creation time. 
		If not, the materialized view will be flagged as unscannable and cannot be queried until REFRESH MATERIALIZED VIEW is used. */
	"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		ADD COLUMN stripe_customer_id TEXT NOT NULL
		CONSTRAINT workspace_uq_stripe_customer_id UNIQUE;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace
		ADD COLUMN primary_payment_method TEXT
		CONSTRAINT workspace_fk_primary_payment_method FOREIGN KEY REFERENCES payment_method(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE deployment_machine_type
		ADD column plan_id UUID
		CONSTRAINT
			deployment_machine_type_uq_plan_id UNIQUE
		CONSTRAINT
			deployment_machine_type_fk_plan_id
		REFERENCES
			plans(id);
	"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
