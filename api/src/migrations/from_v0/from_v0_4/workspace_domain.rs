use api_macros::migrate_query as query;
use api_models::utils::Uuid;

use crate::{utils::settings::Settings, Database};

pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	_config: &Settings,
) -> Result<(), sqlx::Error> {
	query!(
		r#"
		CREATE TABLE domain_tld(
			tld TEXT
				CONSTRAINT domain_tld_pk PRIMARY KEY
				CONSTRAINT domain_tld_chk_is_length_valid CHECK(
					LENGTH(tld) >= 2 AND LENGTH(tld) <= 63
				)
				CONSTRAINT domain_tld_chk_is_tld_valid CHECK(
					tld ~ '^(([a-z0-9])|([a-z0-9][a-z0-9\-\.]*[a-z0-9]))$'
				)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DOMAIN_NAMESERVER_TYPE AS ENUM(
			'internal',
			'external'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE domain
			ALTER COLUMN name SET DATA TYPE TEXT,
			DROP CONSTRAINT domain_chk_name_is_lower_case,
			ADD CONSTRAINT domain_chk_name_is_valid CHECK(
				name ~ '^(([a-z0-9])|([a-z0-9][a-z0-9-]*[a-z0-9]))$' OR
				name LIKE CONCAT(
					'patr-deleted: ',
					REPLACE(id::TEXT, '-', ''),
					'@%'
				)
			),
			ADD COLUMN tld TEXT NOT NULL,
			ADD CONSTRAINT domain_fk_tld FOREIGN KEY(tld)
				REFERENCES domain_tld(tld),
			DROP CONSTRAINT domain_uq_name,
			ADD CONSTRAINT domain_uq_name_tld UNIQUE(name, tld),
			ADD CONSTRAINT domain_chk_max_domain_name_length CHECK(
				(LENGTH(name) + LENGTH(tld)) < 255
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_domain
		RENAME CONSTRAINT workspace_domain_chk_dmn_typ
		TO workspace_domain_chk_domain_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE workspace_domain
			ALTER COLUMN is_verified
				DROP DEFAULT,
			ADD COLUMN nameserver_type
				DOMAIN_NAMESERVER_TYPE NOT NULL,
			ADD CONSTRAINT workspace_domain_uq_id_nameserver_type
				UNIQUE(id, nameserver_type);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE patr_controlled_domain(
			domain_id UUID NOT NULL
				CONSTRAINT patr_controlled_domain_pk PRIMARY KEY,
			zone_identifier TEXT NOT NULL,
			nameserver_type DOMAIN_NAMESERVER_TYPE NOT NULL
				CONSTRAINT patr_controlled_domain_chk_nameserver_type CHECK(
					nameserver_type = 'internal'
				),
			CONSTRAINT patr_controlled_domain_fk_domain_id_nameserver_type
				FOREIGN KEY(domain_id, nameserver_type)	REFERENCES
					workspace_domain(id, nameserver_type)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE user_controlled_domain(
			domain_id UUID NOT NULL
				CONSTRAINT user_controlled_domain_pk PRIMARY KEY,
			nameserver_type DOMAIN_NAMESERVER_TYPE NOT NULL
				CONSTRAINT user_controlled_domain_chk_nameserver_type CHECK(
					nameserver_type = 'external'
				),
			CONSTRAINT user_controlled_domain_fk_domain_id_nameserver_type
				FOREIGN KEY(domain_id, nameserver_type)	REFERENCES
					workspace_domain(id, nameserver_type)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DNS_RECORD_TYPE AS ENUM(
			'A',
			'MX',
			'TXT',
			'AAAA',
			'CNAME'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE patr_domain_dns_record(
			id UUID CONSTRAINT patr_domain_dns_record_pk PRIMARY KEY,
			record_identifier TEXT NOT NULL,
			domain_id UUID NOT NULL,
			name TEXT NOT NULL
				CONSTRAINT patr_domain_dns_record_chk_name_is_lower_case CHECK(
					name = LOWER(name)
				)
				CONSTRAINT patr_domain_dns_record_chk_name_is_trimmed CHECK(
					name = TRIM(name)
				),
			type DNS_RECORD_TYPE NOT NULL,
			value TEXT NOT NULL,
			priority INTEGER,
			ttl BIGINT NOT NULL,
			proxied BOOLEAN NOT NULL,
			CONSTRAINT patr_domain_dns_record_fk_domain_id
				FOREIGN KEY(domain_id)
					REFERENCES patr_controlled_domain(domain_id),
			CONSTRAINT patr_domain_dns_record_chk_values_valid CHECK(
				(
					type = 'MX' AND priority IS NOT NULL
				) OR (
					type != 'MX' AND priority IS NULL
				)
			),
			CONSTRAINT
				patr_domain_dns_record_uq_domain_id_name_type_value_priority
					UNIQUE(domain_id, name, type, value, priority)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Add resource type
	const DNS_RECORD: &str = "dnsRecord";
	let resource_type_id = loop {
		let uuid = Uuid::new_v4();

		let exists = query!(
			r#"
			SELECT
				*
			FROM
				resource_type
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
			resource_type
		VALUES
			($1, $2, NULL);
		"#,
		&resource_type_id,
		DNS_RECORD
	)
	.execute(&mut *connection)
	.await?;

	// Insert all new permissions
	for permission in [
		"workspace::domain::dnsRecord::list",
		"workspace::domain::dnsRecord::add",
		"workspace::domain::dnsRecord::edit",
		"workspace::domain::dnsRecord::delete",
	] {
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
				($1, $2, NULL);
			"#,
			&uuid,
			permission,
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		ALTER TABLE patr_domain_dns_record
		ADD CONSTRAINT patr_domain_dns_record_fk_id
		FOREIGN KEY(id) REFERENCES resource(id);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE personal_domain
		RENAME CONSTRAINT personal_domain_chk_dmn_typ
		TO personal_domain_chk_domain_type;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}
