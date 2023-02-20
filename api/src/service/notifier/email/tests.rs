use std::sync::Once;

use api_models::{
	models::workspace::{
		billing::{
			Address,
			DatabaseUsage,
			DeploymentBill,
			DeploymentUsage,
			DockerRepositoryUsage,
			DomainPlan,
			DomainUsage,
			ManagedUrlUsage,
			PatrDatabaseUsage,
			SecretUsage,
			StaticSitePlan,
			StaticSiteUsage,
			VolumeUsage,
			WorkspaceBillBreakdown,
		},
		infrastructure::list_all_deployment_machine_type::DeploymentMachineType,
	},
	utils::{DateTime, Uuid},
};
use lettre::{
	transport::smtp::authentication::Credentials,
	AsyncSmtpTransport,
	AsyncTransport,
	Message,
	Tokio1Executor,
};

use super::{
	AddEmailVerificationEmail,
	BillNotPaidDeleteResourcesEmail,
	BillPaidSuccessfullyEmail,
	BillPaidUsingCreditsEmail,
	BillPaymentFailedReminderEmail,
	CardNotAddedReminderEmail,
	DomainUnverified,
	DomainVerified,
	ForgotPasswordEmail,
	PartialPaymentSuccessEmail,
	PasswordChangedEmail,
	PasswordResetEmail,
	PaymentFailureInvoiceEmail,
	PaymentSuccessInvoiceEmail,
	PurchaseCreditsSuccessEmail,
	RecoveryNotificationEmail,
	RepositoryStorageLimitExceedEmail,
	ResourceDeletedEmail,
	SignUpCompletedEmail,
	UserSignUpVerificationEmail,
};
use crate::{
	models::EmailTemplate,
	utils::{
		handlebar_registry::{
			get_handlebar_registry,
			initialize_handlebar_registry,
		},
		Error,
	},
};

static INIT: Once = Once::new();

// inorder to send real email for testing, run cargo test
// with the following env variables
//
// SEND_TEST_EMAIL=true
// EMAIL_CRED_USERNAME=<vicara email>
// EMAIL_CRED_PASSWORD=<vicara password>
// EMAIL_FROM=<vicara email>
// EMAIL_TO=<vicara email>
async fn send_email<TEmail>(body: TEmail) -> Result<(), Error>
where
	TEmail: EmailTemplate,
{
	INIT.call_once(initialize_handlebar_registry);
	let handlebar = get_handlebar_registry();

	let send_test_email = std::env::var("SEND_TEST_EMAIL")
		.unwrap_or_else(|_| "false".to_string())
		.parse()
		.unwrap_or_default();

	if send_test_email {
		println!("sending real email for testing");

		let username = std::env::var("EMAIL_CRED_USERNAME")?;
		let password = std::env::var("EMAIL_CRED_PASSWORD")?;
		let from = std::env::var("EMAIL_FROM")?;
		let to = std::env::var("EMAIL_TO")?;

		let message = Message::builder()
			.from(from.parse()?)
			.to(to.parse()?)
			.subject("Patr email testing")
			.multipart(body.render_body(handlebar).await?)?;

		let credentials = Credentials::new(username, password);

		let response = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(
			"smtp.zoho.com",
		)?
		.credentials(credentials)
		.port(587)
		.build::<Tokio1Executor>()
		.send(message)
		.await?;

		if !response.is_positive() {
			println!("{response:#?}");
			return Err(Error::empty().body("Negative email response"));
		}
	} else {
		body.render_body(handlebar).await?;
	}

	Ok(())
}

#[tokio::test]
async fn test_user_sign_up_verification_email() -> Result<(), Error> {
	send_email(UserSignUpVerificationEmail {
		username: "username".to_owned(),
		otp: "otp".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_forgot_password_email() -> Result<(), Error> {
	send_email(ForgotPasswordEmail {
		otp: "otp".to_owned(),
		username: "username".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_password_reset_email() -> Result<(), Error> {
	send_email(PasswordResetEmail {
		username: "username".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_password_changed_email() -> Result<(), Error> {
	send_email(PasswordChangedEmail {
		username: "username".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_sign_up_completed_email() -> Result<(), Error> {
	send_email(SignUpCompletedEmail {
		username: "username".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_recovery_notification_email() -> Result<(), Error> {
	send_email(RecoveryNotificationEmail {
		username: "username".to_owned(),
		recovery_email: "recoveryEmail".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_add_email_verification_email() -> Result<(), Error> {
	send_email(AddEmailVerificationEmail {
		otp: "otp".to_owned(),
		username: "username".to_owned(),
		recovery_email: "recoveryEmail".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_bill_not_paid_delete_resources_email() -> Result<(), Error> {
	send_email(BillNotPaidDeleteResourcesEmail {
		username: "username".to_owned(),
		workspace_name: "workspaceName".to_owned(),
		month: 4,
		year: 2014,
		total_bill: 567,
	})
	.await
}

#[tokio::test]
async fn test_bill_payment_failed_reminder_email() -> Result<(), Error> {
	send_email(BillPaymentFailedReminderEmail {
		username: "username".to_owned(),
		workspace_name: "workspaceName".to_owned(),
		month: 8,
		year: 2351,
		total_bill: 1234124,
		deadline: "deadline".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_card_not_added_reminder_email() -> Result<(), Error> {
	send_email(CardNotAddedReminderEmail {
		username: "username".to_owned(),
		workspace_name: "workspaceName".to_owned(),
		month: 8,
		year: 2351,
		total_bill: 1234124,
		deadline: "deadline".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_bill_paid_successfully_email() -> Result<(), Error> {
	send_email(BillPaidSuccessfullyEmail {
		username: "username".to_owned(),
		workspace_name: "workspaceName".to_owned(),
		month: 7,
		year: 2102,
		card_amount_deducted: 24356,
	})
	.await
}

#[tokio::test]
async fn test_payment_failure_invoice_email() -> Result<(), Error> {
	send_email(PaymentFailureInvoiceEmail {
		username: "username".to_owned(),
		workspace_name: "workspaceName".to_owned(),
		bill_breakdown: WorkspaceBillBreakdown {
			year: 2022,
			month: 5,
			total_charge: 22400,
			deployment_charge: 3200,
			deployment_usage: vec![DeploymentUsage {
				name: "test-deplo".to_string(),
				deployment_id: Uuid::parse_str(
					"d5727fb4-9e6b-43df-8a46-0c698340fffb",
				)
				.unwrap(),
				is_deleted: false,
				deployment_bill: vec![DeploymentBill {
					start_time: DateTime::default(),
					stop_time: Some(DateTime::default()),
					machine_type: DeploymentMachineType {
						id: Uuid::parse_str(
							"d5727fb4-9e6b-43df-8a46-0c698340fffb",
						)
						.unwrap(),
						cpu_count: 1,
						memory_count: 2,
					},
					amount: 3200,
					num_instances: 2,
					hours: 720,
					monthly_charge: 3200,
				}],
			}],
			volume_charge: 3200,
			volume_usage: vec![VolumeUsage {
				start_time: DateTime::default(),
				stop_time: Some(DateTime::default()),
				storage: 10,
				number_of_volume: 5,
				hours: 720,
				amount: 3200,
				monthly_charge: 3200,
			}],
			database_charge: 3200,
			database_usage: vec![DatabaseUsage {
				start_time: DateTime::default(),
				deletion_time: Some(DateTime::default()),
				database_id: Uuid::parse_str(
					"d5727fb4-9e6b-43df-8a46-0c698340fffb",
				)
				.unwrap(),
				name: "mydb".to_string(),
				hours: 720,
				amount: 3200,
				is_deleted: false,
				monthly_charge: 3200,
				plan: "micro".to_string(),
			}],
			patr_database_charge: 3200,
			patr_database_usage: vec![PatrDatabaseUsage {
				start_time: DateTime::default(),
				deletion_time: Some(DateTime::default()),
				database_id: Uuid::parse_str(
					"d5727fb4-9e6b-43df-8a46-0c698340fffb",
				)
				.unwrap(),
				name: "mydb".to_string(),
				hours: 720,
				amount: 3200,
				is_deleted: false,
				monthly_charge: 3200,
				plan: "micro".to_string(),
			}],
			static_site_charge: 3200,
			static_site_usage: vec![StaticSiteUsage {
				start_time: DateTime::default(),
				stop_time: Some(DateTime::default()),
				plan: StaticSitePlan::Free,
				hours: 720,
				amount: 3200,
				monthly_charge: 3200,
			}],
			domain_charge: 3200,
			domain_usage: vec![DomainUsage {
				start_time: DateTime::default(),
				stop_time: Some(DateTime::default()),
				plan: DomainPlan::Free,
				hours: 720,
				amount: 3200,
				monthly_charge: 3200,
			}],
			managed_url_charge: 3200,
			managed_url_usage: vec![ManagedUrlUsage {
				start_time: DateTime::default(),
				stop_time: Some(DateTime::default()),
				plan: "overused".to_string(),
				hours: 720,
				amount: 3200,
				monthly_charge: 3200,
			}],
			secret_charge: 3200,
			secret_usage: vec![SecretUsage {
				start_time: DateTime::default(),
				stop_time: Some(DateTime::default()),
				plan: "overused".to_string(),
				hours: 720,
				amount: 3200,
				monthly_charge: 3200,
			}],
			docker_repository_charge: 3200,
			docker_repository_usage: vec![DockerRepositoryUsage {
				start_time: DateTime::default(),
				stop_time: Some(DateTime::default()),
				plan: "overused".to_string(),
				hours: 720,
				amount: 3200,
				monthly_charge: 3200,
			}],
		},
		billing_address: Address {
			first_name: String::from("John"),
			last_name: String::from("Patr"),
			address_line_1: "221B Baker St, Marylebone".to_string(),
			address_line_2: None,
			address_line_3: None,
			city: "London".to_string(),
			state: "Lincolnshire".to_string(),
			zip: "NW1 6XE".to_string(),
			country: "UK".to_string(),
		},
	})
	.await
}

#[tokio::test]
async fn test_payment_success_invoice_email() -> Result<(), Error> {
	send_email(PaymentSuccessInvoiceEmail {
		username: "username".to_owned(),
		workspace_name: "workspaceName".to_owned(),
		bill_breakdown: WorkspaceBillBreakdown {
			year: 2022,
			month: 5,
			total_charge: 22400,
			deployment_charge: 3200,
			deployment_usage: vec![DeploymentUsage {
				name: "test-deplo".to_string(),
				deployment_id: Uuid::parse_str(
					"d5727fb4-9e6b-43df-8a46-0c698340fffb",
				)
				.unwrap(),
				is_deleted: false,
				deployment_bill: vec![DeploymentBill {
					start_time: DateTime::default(),
					stop_time: Some(DateTime::default()),
					machine_type: DeploymentMachineType {
						id: Uuid::parse_str(
							"d5727fb4-9e6b-43df-8a46-0c698340fffb",
						)
						.unwrap(),
						cpu_count: 1,
						memory_count: 2,
					},
					amount: 3200,
					num_instances: 2,
					hours: 720,
					monthly_charge: 3200,
				}],
			}],
			volume_charge: 3200,
			volume_usage: vec![VolumeUsage {
				start_time: DateTime::default(),
				stop_time: Some(DateTime::default()),
				storage: 10,
				number_of_volume: 5,
				hours: 720,
				amount: 3200,
				monthly_charge: 3200,
			}],
			database_charge: 3200,
			database_usage: vec![DatabaseUsage {
				start_time: DateTime::default(),
				deletion_time: Some(DateTime::default()),
				database_id: Uuid::parse_str(
					"d5727fb4-9e6b-43df-8a46-0c698340fffb",
				)
				.unwrap(),
				name: "mydb".to_string(),
				hours: 720,
				amount: 3200,
				is_deleted: false,
				monthly_charge: 3200,
				plan: "micro".to_string(),
			}],
			patr_database_charge: 3200,
			patr_database_usage: vec![PatrDatabaseUsage {
				start_time: DateTime::default(),
				deletion_time: Some(DateTime::default()),
				database_id: Uuid::parse_str(
					"d5727fb4-9e6b-43df-8a46-0c698340fffb",
				)
				.unwrap(),
				name: "mydb".to_string(),
				hours: 720,
				amount: 3200,
				is_deleted: false,
				monthly_charge: 3200,
				plan: "micro".to_string(),
			}],
			static_site_charge: 3200,
			static_site_usage: vec![StaticSiteUsage {
				start_time: DateTime::default(),
				stop_time: Some(DateTime::default()),
				plan: StaticSitePlan::Free,
				hours: 720,
				amount: 3200,
				monthly_charge: 3200,
			}],
			domain_charge: 3200,
			domain_usage: vec![DomainUsage {
				start_time: DateTime::default(),
				stop_time: Some(DateTime::default()),
				plan: DomainPlan::Free,
				hours: 720,
				amount: 3200,
				monthly_charge: 3200,
			}],
			managed_url_charge: 3200,
			managed_url_usage: vec![ManagedUrlUsage {
				start_time: DateTime::default(),
				stop_time: Some(DateTime::default()),
				plan: "overused".to_string(),
				hours: 720,
				amount: 3200,
				monthly_charge: 3200,
			}],
			secret_charge: 3200,
			secret_usage: vec![SecretUsage {
				start_time: DateTime::default(),
				stop_time: Some(DateTime::default()),
				plan: "overused".to_string(),
				hours: 720,
				amount: 3200,
				monthly_charge: 3200,
			}],
			docker_repository_charge: 3200,
			docker_repository_usage: vec![DockerRepositoryUsage {
				start_time: DateTime::default(),
				stop_time: Some(DateTime::default()),
				plan: "overused".to_string(),
				hours: 720,
				amount: 3200,
				monthly_charge: 3200,
			}],
		},
		billing_address: Address {
			first_name: String::from("John"),
			last_name: String::from("Patr"),
			address_line_1: "221B Baker St, Marylebone".to_string(),
			address_line_2: None,
			address_line_3: None,
			city: "London".to_string(),
			state: "Lincolnshire".to_string(),
			zip: "NW1 6XE".to_string(),
			country: "UK".to_string(),
		},
		credits_deducted: 25443,
		card_amount_deducted: 123423,
		credits_remaining: 45234,
	})
	.await
}

#[tokio::test]
async fn test_resource_deleted_email() -> Result<(), Error> {
	send_email(ResourceDeletedEmail {
		workspace_name: "workspaceName".to_owned(),
		resource_name: "resourceName".to_owned(),
		username: "username".to_owned(),
		deleted_by: "deletedBy".to_owned(),
		resource_type: "resourceType".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_domain_unverified_email() -> Result<(), Error> {
	send_email(DomainUnverified {
		domain_name: "domainName".to_owned(),
		domain_id: "domainId".to_owned(),
		username: "username".to_owned(),
		is_internal: false,
		deadline_limit: 15,
	})
	.await
}

#[tokio::test]
async fn test_domain_verified_email() -> Result<(), Error> {
	send_email(DomainVerified {
		domain_name: "domainName".to_owned(),
		username: "username".to_owned(),
		domain_id: "domainId".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_repo_storage_limit_exceed_email() -> Result<(), Error> {
	send_email(RepositoryStorageLimitExceedEmail {
		username: "username".to_string(),
		workspace_name: "workspace_name".to_string(),
		repository_name: "registry.patr.cloud/workspace_id/repository_name"
			.to_string(),
		tag: "tag".to_string(),
		digest: "digest".to_string(),
		ip_address: "ip_address".to_string(),
	})
	.await
}

#[tokio::test]
async fn test_purchase_credits_success_email() -> Result<(), Error> {
	send_email(PurchaseCreditsSuccessEmail {
		username: "username".to_owned(),
		workspace_name: "workspace_name".to_owned(),
		credits_purchased: 23452345,
	})
	.await
}

#[tokio::test]
async fn test_bill_payed_using_credits_email() -> Result<(), Error> {
	send_email(BillPaidUsingCreditsEmail {
		username: "username".to_owned(),
		workspace_name: "workspace_name".to_owned(),
		total_bill: 500,
		bill_remaining: 450,
		credits_remaining: 0,
	})
	.await
}

#[tokio::test]
async fn test_partial_payment_success_email() -> Result<(), Error> {
	send_email(PartialPaymentSuccessEmail {
		username: "username".to_owned(),
		workspace_name: "workspace_name".to_owned(),
		total_bill: 500,
		amount_paid: 499,
		bill_remaining: 450,
		credits_remaining: 0,
	})
	.await
}
