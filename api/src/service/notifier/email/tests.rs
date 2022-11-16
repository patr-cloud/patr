use std::{collections::BTreeMap, sync::Once};

use api_models::{
	models::workspace::{
		billing::{
			Address,
			DatabaseUsage,
			DeploymentBills,
			DeploymentUsage,
			DockerRepositoryUsage,
			DomainPlan,
			DomainUsage,
			ManagedUrlUsage,
			SecretUsage,
			StaticSitePlan,
			StaticSiteUsage,
			WorkspaceBillBreakdown,
		},
		infrastructure::list_all_deployment_machine_type::DeploymentMachineType,
	},
	utils::Uuid,
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
	BillPaymentFailedReminderEmail,
	CardNotAddedReminderEmail,
	DomainUnverified,
	DomainVerified,
	ForgotPasswordEmail,
	PasswordChangedEmail,
	PasswordResetEmail,
	PaymentFailureInvoiceEmail,
	PaymentSuccessInvoiceEmail,
	RecoveryNotificationEmail,
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
	INIT.call_once(|| initialize_handlebar_registry());
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
async fn test_user_sign_up_verification_email_email() -> Result<(), Error> {
	send_email(UserSignUpVerificationEmail {
		username: "username".to_owned(),
		otp: "otp".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_forgot_password_email_email() -> Result<(), Error> {
	send_email(ForgotPasswordEmail {
		otp: "otp".to_owned(),
		username: "username".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_password_reset_email_email() -> Result<(), Error> {
	send_email(PasswordResetEmail {
		username: "username".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_password_changed_email_email() -> Result<(), Error> {
	send_email(PasswordChangedEmail {
		username: "username".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_sign_up_completed_email_email() -> Result<(), Error> {
	send_email(SignUpCompletedEmail {
		username: "username".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_recovery_notification_email_email() -> Result<(), Error> {
	send_email(RecoveryNotificationEmail {
		username: "username".to_owned(),
		recoveryEmail: "recoveryEmail".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_add_email_verification_email_email() -> Result<(), Error> {
	send_email(AddEmailVerificationEmail {
		otp: "otp".to_owned(),
		username: "username".to_owned(),
		recoveryEmail: "recoveryEmail".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_bill_not_paid_delete_resources_email_email() -> Result<(), Error>
{
	send_email(BillNotPaidDeleteResourcesEmail {
		username: "username".to_owned(),
		workspaceName: "workspaceName".to_owned(),
		month: 4,
		year: 2014,
		totalBill: 567,
	})
	.await
}

#[tokio::test]
async fn test_bill_payment_failed_reminder_email_email() -> Result<(), Error> {
	send_email(BillPaymentFailedReminderEmail {
		username: "username".to_owned(),
		workspaceName: "workspaceName".to_owned(),
		month: 8,
		year: 2351,
		totalBill: 1234124,
		deadline: "deadline".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_card_not_added_reminder_email_email() -> Result<(), Error> {
	send_email(CardNotAddedReminderEmail {
		username: "username".to_owned(),
		workspaceName: "workspaceName".to_owned(),
		month: 8,
		year: 2351,
		totalBill: 1234124,
		deadline: "deadline".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_bill_paid_successfully_email_email() -> Result<(), Error> {
	send_email(BillPaidSuccessfullyEmail {
		username: "username".to_owned(),
		workspaceName: "workspaceName".to_owned(),
		month: 7,
		year: 2102,
		cardAmountDeducted: 24356,
	})
	.await
}

#[tokio::test]
async fn test_payment_failure_invoice_email_email() -> Result<(), Error> {
	send_email(PaymentFailureInvoiceEmail {
		username: "username".to_owned(),
		workspaceName: "workspaceName".to_owned(),
		billBreakdown: WorkspaceBillBreakdown {
			year: 2022,
			month: 5,
			total_charge: 22400,
			deployment_charge: 3200,
			deployment_usage: BTreeMap::from([(
				Uuid::parse_str("d5727fb4-9e6b-43df-8a46-0c698340fffb")
					.unwrap(),
				DeploymentUsage {
					name: "test-deplo".to_string(),
					bill_items: vec![DeploymentBills {
						machine_type: DeploymentMachineType {
							id: Uuid::parse_str(
								"d5727fb4-9e6b-43df-8a46-0c698340fffb",
							)
							.unwrap(),
							cpu_count: 1,
							memory_count: 2,
						},
						num_instances: 2,
						hours: 720,
						amount: 3200,
					}],
				},
			)]),
			database_charge: 3200,
			database_usage: BTreeMap::from([(
				Uuid::parse_str("d5727fb4-9e6b-43df-8a46-0c698340fffb")
					.unwrap(),
				DatabaseUsage {
					name: "mydb".to_string(),
					hours: 720,
					amount: 3200,
				},
			)]),
			static_site_charge: 3200,
			static_site_usage: BTreeMap::from([(
				StaticSitePlan::Pro,
				StaticSiteUsage {
					hours: 720,
					amount: 3200,
				},
			)]),
			domain_charge: 3200,
			domain_usage: BTreeMap::from([(
				DomainPlan::Free,
				DomainUsage {
					hours: 720,
					amount: 3200,
				},
			)]),
			managed_url_charge: 3200,
			managed_url_usage: BTreeMap::from([(
				5,
				ManagedUrlUsage {
					hours: 720,
					amount: 3200,
				},
			)]),
			secret_charge: 3200,
			secret_usage: BTreeMap::from([(
				5,
				SecretUsage {
					hours: 720,
					amount: 3200,
				},
			)]),
			docker_repository_charge: 3200,
			docker_repository_usage: vec![DockerRepositoryUsage {
				storage: 10,
				hours: 720,
				amount: 3200,
			}],
		},
		billingAddress: Address {
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
async fn test_payment_success_invoice_email_email() -> Result<(), Error> {
	send_email(PaymentSuccessInvoiceEmail {
		username: "username".to_owned(),
		workspaceName: "workspaceName".to_owned(),
		billBreakdown: WorkspaceBillBreakdown {
			year: 2022,
			month: 5,
			total_charge: 22400,
			deployment_charge: 3200,
			deployment_usage: BTreeMap::from([(
				Uuid::parse_str("d5727fb4-9e6b-43df-8a46-0c698340fffb")
					.unwrap(),
				DeploymentUsage {
					name: "test-deplo".to_string(),
					bill_items: vec![DeploymentBills {
						machine_type: DeploymentMachineType {
							id: Uuid::parse_str(
								"d5727fb4-9e6b-43df-8a46-0c698340fffb",
							)
							.unwrap(),
							cpu_count: 1,
							memory_count: 2,
						},
						num_instances: 2,
						hours: 720,
						amount: 3200,
					}],
				},
			)]),
			database_charge: 3200,
			database_usage: BTreeMap::from([(
				Uuid::parse_str("d5727fb4-9e6b-43df-8a46-0c698340fffb")
					.unwrap(),
				DatabaseUsage {
					name: "mydb".to_string(),
					hours: 720,
					amount: 3200,
				},
			)]),
			static_site_charge: 3200,
			static_site_usage: BTreeMap::from([(
				StaticSitePlan::Pro,
				StaticSiteUsage {
					hours: 720,
					amount: 3200,
				},
			)]),
			domain_charge: 3200,
			domain_usage: BTreeMap::from([(
				DomainPlan::Free,
				DomainUsage {
					hours: 720,
					amount: 3200,
				},
			)]),
			managed_url_charge: 3200,
			managed_url_usage: BTreeMap::from([(
				5,
				ManagedUrlUsage {
					hours: 720,
					amount: 3200,
				},
			)]),
			secret_charge: 3200,
			secret_usage: BTreeMap::from([(
				5,
				SecretUsage {
					hours: 720,
					amount: 3200,
				},
			)]),
			docker_repository_charge: 3200,
			docker_repository_usage: vec![DockerRepositoryUsage {
				storage: 10,
				hours: 720,
				amount: 3200,
			}],
		},
		billingAddress: Address {
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
		creditsDeducted: 25443,
		cardAmountDeducted: 123423,
		creditsRemaining: 45234,
	})
	.await
}

#[tokio::test]
async fn test_resource_deleted_email_email() -> Result<(), Error> {
	send_email(ResourceDeletedEmail {
		workspaceName: "workspaceName".to_owned(),
		resourceName: "resourceName".to_owned(),
		username: "username".to_owned(),
		deletedBy: "deletedBy".to_owned(),
		resourceType: "resourceType".to_owned(),
	})
	.await
}

#[tokio::test]
async fn test_domain_unverified_email() -> Result<(), Error> {
	send_email(DomainUnverified {
		domainName: "domainName".to_owned(),
		domainId: "domainId".to_owned(),
		username: "username".to_owned(),
		isInternal: false,
	})
	.await
}

#[tokio::test]
async fn test_domain_verified_email() -> Result<(), Error> {
	send_email(DomainVerified {
		domainName: "domainName".to_owned(),
		username: "username".to_owned(),
		domainId: "domainId".to_owned(),
	})
	.await
}
