#[cfg(test)]
mod tests {
	use api_models::{
		models::workspace::domain::DomainNameserverType,
		utils::{ResourceType as Resource, Uuid},
	};
	use chrono::Utc;

	use crate::{
		db::*,
		models::rbac,
		test::{deinit_test, user_constaints},
		utils::Error,
	};

	#[tokio::test]
	async fn domain_test() -> Result<(), Error> {
		let app = user_constaints().await?;
		let mut connection = app.database.acquire().await?;
		let workspace_id =
			get_workspace_by_name(&mut connection, "test-workspace")
				.await?
				.unwrap()
				.id;
		// Create domain
		let personal_domain_name = "test";
		let tld = "com";
		let internal_nameserver_type = DomainNameserverType::Internal;
		let cloudflare_worker_route_id = "123".to_owned();

		// Internal domain
		let internal_domain_id =
			generate_new_domain_id(&mut connection).await?;
		let zone_identifier = "test".to_owned();
		create_resource(
			&mut connection,
			&internal_domain_id,
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::DOMAIN)
				.unwrap(),
			&workspace_id,
			&Utc::now(),
		)
		.await?;
		sqlx::query("INSERT INTO domain_tld VALUES ('com')")
			.execute(&mut connection)
			.await?;
		create_generic_domain(
			&mut connection,
			&internal_domain_id,
			personal_domain_name,
			tld,
			&Resource::Business,
		)
		.await?;
		add_to_workspace_domain(
			&mut connection,
			&internal_domain_id,
			&internal_nameserver_type,
			&cloudflare_worker_route_id,
		)
		.await?;
		add_patr_controlled_domain(
			&mut connection,
			&internal_domain_id,
			&zone_identifier,
		)
		.await?;

		// Test
		let workspace_domain_output =
			get_domains_for_workspace(&mut connection, &workspace_id).await?;
		let workspace_id_domain_output =
			get_workspace_domain_by_id(&mut connection, &internal_domain_id)
				.await?
				.unwrap();
		let internal_domain_output = get_patr_controlled_domain_by_id(
			&mut connection,
			&internal_domain_id,
		)
		.await?
		.unwrap();
		assert_eq!(internal_domain_output.domain_id, internal_domain_id);
		assert_eq!(
			internal_domain_output.nameserver_type,
			internal_nameserver_type
		);
		assert_eq!(internal_domain_output.zone_identifier, zone_identifier);
		assert_eq!(workspace_domain_output[0].id, internal_domain_id);
		assert_eq!(workspace_domain_output[0].name, "test.com");
		assert_eq!(
			workspace_domain_output[0].nameserver_type,
			internal_nameserver_type
		);
		assert_eq!(
			workspace_domain_output[0].cloudflare_worker_route_id,
			cloudflare_worker_route_id
		);
		assert_eq!(workspace_domain_output[0].domain_type, Resource::Business); //
		assert_eq!(workspace_domain_output[0].is_verified, false);
		assert_eq!(workspace_id_domain_output.id, internal_domain_id);
		assert_eq!(workspace_id_domain_output.name, "test.com");
		assert_eq!(
			workspace_id_domain_output.nameserver_type,
			internal_nameserver_type
		);
		assert_eq!(
			workspace_id_domain_output.cloudflare_worker_route_id,
			cloudflare_worker_route_id
		);
		assert_eq!(workspace_id_domain_output.domain_type, Resource::Business); //
		assert_eq!(workspace_id_domain_output.is_verified, false);

		// External domain
		let external_domain_id =
			generate_new_domain_id(&mut connection).await?;
		let external_domain_name = "external".to_owned();
		let external_nameserver_type = DomainNameserverType::External;
		create_resource(
			&mut connection,
			&external_domain_id,
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::DOMAIN)
				.unwrap(),
			&workspace_id,
			&Utc::now(),
		)
		.await?;
		create_generic_domain(
			&mut connection,
			&external_domain_id,
			&external_domain_name,
			tld,
			&Resource::Business,
		)
		.await?;
		add_to_workspace_domain(
			&mut connection,
			&external_domain_id,
			&external_nameserver_type,
			&cloudflare_worker_route_id,
		)
		.await?;
		add_user_controlled_domain(&mut connection, &external_domain_id)
			.await?;
		// Test
		let domain_name_output =
			get_domain_by_name(&mut connection, "external.com")
				.await?
				.unwrap();
		assert_eq!(domain_name_output.id, external_domain_id);
		assert_eq!(domain_name_output.name, "external.com");
		assert_eq!(domain_name_output.r#type, Resource::Business);

		// Verify
		update_workspace_domain_status(
			&mut connection,
			&internal_domain_id,
			true,
			&Utc::now(),
		)
		.await?;
		let verified_output = get_all_verified_domains(&mut connection).await?;
		// Test
		assert_eq!(verified_output[0].0.is_verified, true);

		// Add DNS records
		let record_id = generate_new_resource_id(&mut connection).await?;
		create_resource(
			&mut connection,
			&record_id,
			rbac::RESOURCE_TYPES
				.get()
				.unwrap()
				.get(rbac::resource_types::DNS_RECORD)
				.unwrap(),
			&workspace_id,
			&Utc::now(),
		)
		.await?;
		let dns_name = "@".to_owned();
		let dns_value = "record".to_owned();
		let dns_identifier = Uuid::new_v4();
		create_patr_domain_dns_record(
			&mut connection,
			&record_id,
			Uuid::nil().as_str(),
			&internal_domain_id,
			&dns_name,
			&DnsRecordType::MX,
			&dns_value,
			Some(2),
			10 as i64,
			None,
		)
		.await?;
		// Test
		let record_output = get_dns_record_by_id(&mut connection, &record_id)
			.await?
			.unwrap();
		assert_eq!(record_output.id, record_id);
		assert_eq!(record_output.domain_id, internal_domain_id);
		assert_eq!(record_output.name, dns_name);
		assert_eq!(record_output.priority, Some(2));
		assert_eq!(record_output.proxied, None);
		assert_eq!(record_output.record_identifier, Uuid::nil().as_str());
		assert_eq!(record_output.ttl, 10 as i64);
		assert_eq!(record_output.r#type, DnsRecordType::MX);
		assert_eq!(record_output.value, dns_value);

		// Update DNS record
		let updated_dns_value = "new record".to_owned();
		update_patr_domain_dns_record(
			&mut connection,
			&record_id,
			Some(&updated_dns_value),
			Some(3),
			Some(5),
			None,
		)
		.await?;
		update_dns_record_identifier(
			&mut connection,
			&record_id,
			dns_identifier.as_str(),
		)
		.await?;
		// Test
		let update_record_output =
			get_dns_record_by_id(&mut connection, &record_id)
				.await?
				.unwrap();
		assert_eq!(update_record_output.priority, Some(3));
		assert_eq!(update_record_output.proxied, None);
		assert_eq!(
			update_record_output.record_identifier,
			dns_identifier.to_string()
		);
		assert_eq!(update_record_output.ttl, 5 as i64);
		assert_eq!(update_record_output.value, updated_dns_value);

		// Delete DNS record
		delete_patr_controlled_dns_record(&mut connection, &record_id).await?;
		let delete_record_output =
			get_dns_records_by_domain_id(&mut connection, &internal_domain_id)
				.await?;
		// Test
		assert_eq!(0, delete_record_output.len());

		// Delete a domain
		mark_domain_as_deleted(
			&mut connection,
			&internal_domain_id,
			&Utc::now(),
		)
		.await?;
		mark_domain_as_deleted(
			&mut connection,
			&external_domain_id,
			&Utc::now(),
		)
		.await?;
		// Test
		let delete_domain_output =
			get_domains_for_workspace(&mut connection, &workspace_id).await?;
		assert_eq!(0, delete_domain_output.len());

		deinit_test(app.config.database.database).await?;
		Ok(())
	}
}
