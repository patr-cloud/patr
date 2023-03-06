use std::time::{SystemTime, UNIX_EPOCH};

use api_macros::migrate_query as query;
use api_models::utils::Uuid;
use cloudflare::{
	endpoints::{
		dns::{DnsContent, ListDnsRecords, ListDnsRecordsParams},
		zone::{ListZones, ListZonesParams},
	},
	framework::{
		async_api::Client,
		auth::Credentials,
		Environment,
		HttpApiClientConfig,
	},
};
use sqlx::Row;

use crate::{
	utils::{settings::Settings, Error},
	Database,
};

pub async fn migrate(
	connection: &mut <Database as sqlx::Database>::Connection,
	config: &Settings,
) -> Result<(), Error> {
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
			ADD COLUMN tld TEXT,
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
				CONSTRAINT patr_domain_dns_record_chk_name_is_valid CHECK(
					name ~ '^(\*)|((\*\.)?(([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_])\.)*([a-z0-9_]|[a-z0-9_][a-z0-9_\-]*[a-z0-9_]))$' OR
					name = '@'
				),
			type DNS_RECORD_TYPE NOT NULL,
			value TEXT NOT NULL,
			priority INTEGER,
			ttl BIGINT NOT NULL,
			proxied BOOLEAN,
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
					UNIQUE(domain_id, name, type, value, priority),
			CONSTRAINT patr_domain_dns_record_chk_proxied_is_valid CHECK(
				(
					(type = 'A' OR type = 'AAAA' OR type = 'CNAME') AND
					proxied IS NOT NULL
				) OR
				(
					(type = 'MX' OR type = 'TXT') AND
					proxied IS NULL
				)
			)
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	// Add resource type
	const DNS_RECORD: &str = "dnsRecord";
	let dns_record_resource_type_id = loop {
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
		&dns_record_resource_type_id,
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

	// Update domain TLD list
	let data =
		reqwest::get("https://data.iana.org/TLD/tlds-alpha-by-domain.txt")
			.await?
			.text()
			.await?;

	let tlds = data
		.split('\n')
		.map(String::from)
		.filter(|tld| {
			!tld.starts_with('#') && !tld.is_empty() && !tld.starts_with("XN--")
		})
		.map(|item| item.to_lowercase())
		.collect::<Vec<String>>();

	for tld in &tlds {
		query!(
			r#"
			INSERT INTO
				domain_tld
			VALUES
				($1)
			ON CONFLICT DO NOTHING;
			"#,
			tld,
		)
		.execute(&mut *connection)
		.await?;
	}

	query!(
		r#"
		UPDATE
			domain
		SET
			tld = (
				SELECT
					tld
				FROM
					domain_tld
				WHERE
					domain.name LIKE CONCAT('%.', domain_tld.tld)
				ORDER BY
					LENGTH(domain_tld.tld) DESC
				LIMIT 1
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		UPDATE
			domain
		SET
			name = REPLACE(name, CONCAT('.', tld), '');
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		ALTER TABLE domain
			ALTER COLUMN tld SET NOT NULL,
			ADD CONSTRAINT domain_chk_name_is_valid CHECK(
				name ~ '^(([a-z0-9])|([a-z0-9][a-z0-9-]*[a-z0-9]))$' OR
				name LIKE CONCAT(
					'patr-deleted: ',
					REPLACE(id::TEXT, '-', ''),
					'@%'
				)
			);
		"#
	)
	.execute(&mut *connection)
	.await?;

	let domain_resource_type_id = query!(
		r#"
		SELECT
			id
		FROM
			resource_type
		WHERE
			name = 'domain';
		"#
	)
	.fetch_one(&mut *connection)
	.await?
	.get::<Uuid, _>("id");

	// Migrate all existing domains and DNS records to the workspace who's
	// super_admin_id is GOD_USER_ID
	let god_user_id = query!(
		r#"
		SELECT
			id
		FROM
			"user"
		ORDER BY
			created
		LIMIT 1;
		"#
	)
	.fetch_optional(&mut *connection)
	.await?;

	let god_user_id = if let Some(id) = god_user_id {
		id.get::<Uuid, _>("id")
	} else {
		// No GOD_USER_ID to migrate to
		return Ok(());
	};

	let workspace = query!(
		r#"
		SELECT
			id
		FROM
			workspace
		WHERE
			super_admin_id = $1;
		"#,
		&god_user_id
	)
	.fetch_optional(&mut *connection)
	.await?;

	let workspace_id = if let Some(workspace) = workspace {
		workspace.get::<Uuid, _>("id")
	} else {
		// Cannot migrate to a workspace
		return Ok(());
	};

	let credentials = Credentials::UserAuthToken {
		token: config.cloudflare.api_token.clone(),
	};

	let client = Client::new(
		credentials,
		HttpApiClientConfig::default(),
		Environment::Production,
	)
	.map_err(|e| {
		log::error!("Failed to create client: {}", e);
		sqlx::Error::WorkerCrashed
	})?;

	let zones = client
		.request(&ListZones {
			params: ListZonesParams {
				per_page: Some(50),
				..Default::default()
			},
		})
		.await?
		.result;

	for zone in zones {
		let zone_identifier = zone.id.as_str();
		let domain_name = zone.name;
		let tld = tlds
			.iter()
			.filter(|tld| domain_name.ends_with(*tld))
			.reduce(|accumulator, item| {
				if accumulator.len() > item.len() {
					accumulator
				} else {
					item
				}
			})
			.unwrap_or_else(|| {
				panic!(
					"unable to find a suitable TLD for domain `{}`",
					domain_name
				)
			});
		let domain = domain_name.replace(&format!(".{}", tld), "");

		// Insert zone
		let domain_id = loop {
			let uuid = Uuid::new_v4();

			// If it exists in the resource table, it can't be used
			// because workspace domains are a resource
			// If it exists in the domain table, it can't be used
			// since personal domains are a type of domains
			let exists = {
				query!(
					r#"
					SELECT
						*
					FROM
						resource
					WHERE
						id = $1;
					"#,
					&uuid
				)
				.fetch_optional(&mut *connection)
				.await?
				.is_some()
			} || {
				query!(
					r#"
					SELECT
						id
					FROM
						domain
					WHERE
						id = $1;
					"#,
					&uuid
				)
				.fetch_optional(&mut *connection)
				.await?
				.is_some()
			};

			if !exists {
				break uuid;
			}
		};

		// add a resource first
		query!(
			r#"
			INSERT INTO
				resource
			VALUES
				($1, $2, $3, $4, $5);
			"#,
			&domain_id,
			format!("Domain: {}", domain_name),
			&domain_resource_type_id,
			&workspace_id,
			SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.expect("Time went backwards. Wtf?")
				.as_millis() as i64,
		)
		.execute(&mut *connection)
		.await?;
		query!(
			r#"
			INSERT INTO
				domain
			VALUES
				($1, $2, 'business', $3);
			"#,
			&domain_id,
			&domain,
			tld
		)
		.execute(&mut *connection)
		.await?;
		query!(
			r#"
			INSERT INTO
				workspace_domain
			VALUES
				($1, 'business', FALSE, 'internal');
			"#,
			&domain_id,
		)
		.execute(&mut *connection)
		.await?;
		query!(
			r#"
			INSERT INTO
				patr_controlled_domain
			VALUES
				($1, $2, 'internal');
			"#,
			&domain_id,
			zone_identifier
		)
		.execute(&mut *connection)
		.await?;

		let dns_records = client
			.request(&ListDnsRecords {
				zone_identifier,
				params: ListDnsRecordsParams {
					per_page: Some(5000),
					..Default::default()
				},
			})
			.await?
			.result;

		for dns_record in dns_records {
			let record_identifier = dns_record.id.as_str();
			let name = dns_record
				.name
				.replace(&domain_name, "")
				.trim_end_matches('.')
				.to_string();
			let name = if name.is_empty() { "@" } else { name.as_str() };
			let (r#type, value, priority, proxied) = match dns_record.content {
				DnsContent::A { content, .. } => {
					("A", content.to_string(), None, Some(dns_record.proxied))
				}
				DnsContent::AAAA { content, .. } => (
					"AAAA",
					content.to_string(),
					None,
					Some(dns_record.proxied),
				),
				DnsContent::CNAME { content, .. } => {
					("CNAME", content, None, Some(dns_record.proxied))
				}
				DnsContent::MX { content, priority } => {
					("MX", content, Some(priority as i32), None)
				}
				DnsContent::TXT { content } => ("TXT", content, None, None),
				_ => continue,
			};
			let ttl = dns_record.ttl as i64;

			let record_id = loop {
				let uuid = Uuid::new_v4();

				let exists = {
					query!(
						r#"
						SELECT
							*
						FROM
							resource
						WHERE
							id = $1;
						"#,
						&uuid
					)
					.fetch_optional(&mut *connection)
					.await?
					.is_some()
				};

				if !exists {
					break uuid;
				}
			};

			// add a resource first
			query!(
				r#"
				INSERT INTO
					resource
				VALUES
					($1, $2, $3, $4, $5);
				"#,
				&record_id,
				&format!("DNS Record `{}.{}`: {}", name, domain_id, r#type),
				&dns_record_resource_type_id,
				&workspace_id,
				SystemTime::now()
					.duration_since(UNIX_EPOCH)
					.expect("Time went backwards. Wtf?")
					.as_millis() as i64,
			)
			.execute(&mut *connection)
			.await?;
			query!(
				r#"
				INSERT INTO
					patr_domain_dns_record
				VALUES
					($1, $2, $3, $4, $5::DNS_RECORD_TYPE, $6, $7, $8, $9);
				"#,
				&record_id,
				record_identifier,
				&domain_id,
				&name,
				&r#type,
				&value,
				&priority,
				ttl,
				proxied,
			)
			.execute(&mut *connection)
			.await?;
		}
	}

	Ok(())
}
