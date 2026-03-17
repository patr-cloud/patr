use cloudflare::{
	endpoints::zones::custom_hostnames::*,
	framework::{
		Environment,
		auth::Credentials,
		client::{ClientConfig, async_api::Client as CloudflareClient},
	},
};
use http::StatusCode;
use models::api::workspace::domain::*;

use crate::prelude::*;

/// The handler to verify a domain in a workspace. This will check if the domain
/// has been verified by checking the DNS records for the required verification
/// record. If the record is found, the domain will be marked as verified.
pub async fn get_verification_records_for_domain(
	AuthenticatedAppRequest {
		request:
			ProcessedApiRequest {
				path:
					GetVerificationRecordsForDomainPath {
						workspace_id: _,
						domain_id,
					},
				query: (),
				headers:
					GetVerificationRecordsForDomainRequestHeaders {
						authorization: _,
						user_agent: _,
					},
				body: GetVerificationRecordsForDomainRequestProcessed,
			},
		database,
		redis: _,
		client_ip: _,
		user_data: _,
		state,
	}: AuthenticatedAppRequest<'_, GetVerificationRecordsForDomainRequest>,
) -> Result<AppResponse<GetVerificationRecordsForDomainRequest>, ErrorType> {
	info!("Starting: Get verification records for domain in workspace");

	let row = query!(
		r#"
		SELECT
			cloudflare_custom_hostname_id,
			CONCAT(name, '.', tld) AS "name!"
		FROM
			workspace_domain
		WHERE
			id = $1 AND
			workspace_domain.deleted IS NULL;
		"#,
		domain_id as _,
	)
	.fetch_one(&mut **database)
	.await?;

	let custom_hostname_id = row.cloudflare_custom_hostname_id;
	let domain_name = row.name;

	let response = CloudflareClient::new(
		Credentials::UserAuthToken {
			token: state.config.cloudflare.api_key.clone(),
		},
		ClientConfig::default(),
		Environment::Custom(state.config.cloudflare.base_url.clone()),
	)?
	.request(&GetCustomHostnameDetails {
		zone_identifier: &state.config.cloudflare.primary_hosted_zone_id,
		custom_hostname_id: &custom_hostname_id,
	})
	.await?
	.result;

	// Usually there's one ownership verification record and two SSL records
	let mut verification_records = Vec::with_capacity(3);

	if let Some(verification) = response.ownership_verification {
		if verification.type_ != "txt" {
			error!(
				"Unexpected ownership verification type `{}` for domain `{}`",
				verification.type_, domain_name
			);
			error!("Ideally this should never happen unless Cloudflare changes their API");
			return Err(ErrorType::server_error(format!(
				"Unexpected ownership verification type: {}",
				verification.type_
			)));
		}
		verification_records.push(PatrDomainDnsRecord {
			name: verification.name,
			r#type: DnsRecordValue::TXT {
				target: verification.value,
			},
			ttl: 0,
		});
	}

	if let Some(validation_records) = response.ssl.and_then(|ssl| ssl.validation_records) {
		let records = validation_records.into_iter().filter_map(|record| {
			Some(PatrDomainDnsRecord {
				name: record.txt_name?,
				r#type: DnsRecordValue::TXT {
					target: record.txt_value?,
				},
				ttl: 0,
			})
		});
		verification_records.extend(records);
	}

	AppResponse::builder()
		.body(GetVerificationRecordsForDomainResponse {
			verification_records,
		})
		.headers(())
		.status_code(StatusCode::OK)
		.build()
		.into_result()
}
