use std::collections::HashMap;

use api_models::utils::Uuid;
use lettre::message::Mailbox;
use serde::Serialize;

use crate::{
	db::{DomainPlan, StaticSitePlan},
	models::{
		billing::{
			DatabaseBill,
			DeploymentBill,
			DockerRepositoryBill,
			DomainBill,
			ManagedUrlBill,
			SecretsBill,
			StaticSiteBill,
		},
		deployment::KubernetesEventData,
		EmailTemplate,
	},
	utils::Error,
};

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/user-sign-up/template.json"]
struct UserSignUpVerificationEmail {
	username: String,
	otp: String,
}

/// # Description
/// This function is used to email the otp to user for account verification
///
/// # Arguments
/// * `email` - Represents an email address with an optional name for the
///   sender/recipient.
/// More info here: [`Mailbox`]
/// * `otp` - a string containing One Time Password to be sent to the user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Mailbox`]: Mailbox
pub async fn send_user_verification_otp(
	email: Mailbox,
	username: &str,
	otp: &str,
) -> Result<(), Error> {
	send_email(
		UserSignUpVerificationEmail {
			username: username.to_string(),
			otp: otp.to_string(),
		},
		email,
		None,
		"Patr verification OTP",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/forgot-password/template.json"]
struct ForgotPasswordEmail {
	otp: String,
	username: String,
}

/// # Description
/// This function is used to email the otp to user for verifiying change in
/// password incase the user forgets the password
///
/// # Arguments
/// * `email` - Represents an email address with an optional name for the
///   sender/recipient.
/// More info here: [`Mailbox`]
/// * `otp` - a string containing One Time Password to be sent to the user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Mailbox`]: Mailbox
pub async fn send_forgot_password_otp(
	email: Mailbox,
	otp: &str,
	username: &str,
) -> Result<(), Error> {
	send_email(
		ForgotPasswordEmail {
			otp: otp.to_string(),
			username: username.to_string(),
		},
		email,
		None,
		"Patr password reset request",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/password-reset-notification/template.json"]
struct PasswordResetEmail {
	username: String,
}

/// # Description
/// This function is used to send the password reset notification
///
/// # Arguments
/// * `email` - Represents an email address with an optional name for the
///   sender/recipient.
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Mailbox`]: Mailbox
pub async fn send_user_reset_password_notification(
	email: Mailbox,
	username: &str,
) -> Result<(), Error> {
	send_email(
		PasswordResetEmail {
			username: username.to_string(),
		},
		email,
		None,
		"Patr successful password change",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/password-changed-notification/template.json"]
struct PasswordChangedEmail {
	username: String,
}

/// # Description
/// This function is used to send the password changed notification
///
/// # Arguments
/// * `email` - Represents an email address with an optional name for the
///   sender/recipient.
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
pub async fn send_password_changed_notification(
	email: Mailbox,
	username: &str,
) -> Result<(), Error> {
	send_email(
		PasswordChangedEmail {
			username: username.to_string(),
		},
		email,
		None,
		"Patr password change",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/sign-up-completed/template.json"]
struct SignUpCompletedEmail {
	username: String,
}

/// # Description
/// This function is used to send the sign up complete notification
///
/// # Arguments
/// * `email` - Represents an email address with an optional name for the
///   sender/recipient.
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
pub async fn send_sign_up_completed_email(
	email: Mailbox,
	username: &str,
) -> Result<(), Error> {
	send_email(
		SignUpCompletedEmail {
			username: username.to_string(),
		},
		email,
		None,
		"Welcome to Patr",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/recovery-email-notification/template.json"]
struct RecoveryNotificationEmail {
	username: String,
}

/// # Description
/// This function is used to send the registration info to back up email of the
/// user
///
/// # Arguments
/// * `email` - Represents an email address with an optional name for the
///   sender/recipient.
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
pub async fn send_recovery_registration_mail(
	email: Mailbox,
	username: &str,
) -> Result<(), Error> {
	send_email(
		RecoveryNotificationEmail {
			username: username.to_string(),
		},
		email,
		None,
		"Welcome to Patr",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/add-new-email-notification/template.json"]
struct AddEmailVerificationEmail {
	otp: String,
	username: String,
}

/// # Description
/// This function is used to email the otp to user for adding a new email
///
/// # Arguments
/// * `email` - Represents an email address with an optional name for the
///   sender/recipient.
/// More info here: [`Mailbox`]
/// * `otp` - a string containing One Time Password to be sent to the user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Mailbox`]: Mailbox
pub async fn send_email_verification_otp(
	email: Mailbox,
	otp: &str,
	username: &str,
) -> Result<(), Error> {
	send_email(
		AddEmailVerificationEmail {
			username: username.to_string(),
			otp: otp.to_string(),
		},
		email,
		None,
		"Patr email verification OTP",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/send-deployment-alert-notification/template.json"]
struct DeploymentAlertEmail {
	workspace_name: String,
	deployment_id: String,
	deployment_name: String,
	message: String,
}

/// # Description
/// This function is used to email alert to the user
///
/// # Arguments
/// * `email` - Represents an email address with an optional name for the
///   sender/recipient.
/// More info here: [`Mailbox`]
/// * `message` - The message to be sent to the user
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Mailbox`]: Mailbox
pub async fn send_alert_email(
	email: Mailbox,
	workspace_name: &str,
	deployment_id: &Uuid,
	deployment_name: &str,
	message: &str,
) -> Result<(), Error> {
	send_email(
		DeploymentAlertEmail {
			message: message.to_string(),
			workspace_name: workspace_name.to_string(),
			deployment_id: deployment_id.to_string(),
			deployment_name: deployment_name.to_string(),
		},
		email,
		None,
		"Patr Deployment alert",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/send-kubernetes-patr-alert-notification/template.json"]
pub struct KubernetesPatrAlertEmail {
	event_data: String,
}

/// # Description
/// This function is used to email alert to patr
///
/// # Arguments
/// * `email` - Represents an email address with an optional name for the
///   sender/recipient.
/// More info here: [`Mailbox`]
/// * `message` - The message to be sent to patr
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// error
///
/// [`Mailbox`]: Mailbox
pub async fn send_alert_email_to_patr(
	email: Mailbox,
	event_data: KubernetesEventData,
) -> Result<(), Error> {
	let event_data = serde_json::to_string(&event_data)?;
	send_email(
		KubernetesPatrAlertEmail { event_data },
		email,
		None,
		"Patr Kubernetes alert",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/invoice-email/template.json"]
struct InvoiceEmail {
	workspace_name: String,
	billing_address: HashMap<String, String>,
	deployment_usages: HashMap<Uuid, DeploymentBill>,
	database_usages: HashMap<Uuid, DatabaseBill>,
	static_site_usages: HashMap<StaticSitePlan, StaticSiteBill>,
	managed_url_usages: HashMap<u64, ManagedUrlBill>,
	docker_repository_usages: Vec<DockerRepositoryBill>,
	domain_usages: HashMap<DomainPlan, DomainBill>,
	secret_usages: HashMap<u64, SecretsBill>,
	total_bill: f64,
	deployment_bill: f64,
	database_bill: f64,
	static_site_bill: f64,
	managed_url_bill: f64,
	docker_repo_bill: f64,
	managed_domain_bill: f64,
	managed_secret_bill: f64,
	month: String,
	year: i32,
}

#[allow(clippy::too_many_arguments)]
pub async fn send_invoice_email(
	email: Mailbox,
	workspace_name: String,
	billing_address: HashMap<String, String>,
	deployment_usages: HashMap<Uuid, DeploymentBill>,
	database_usages: HashMap<Uuid, DatabaseBill>,
	static_site_usages: HashMap<StaticSitePlan, StaticSiteBill>,
	managed_url_usages: HashMap<u64, ManagedUrlBill>,
	docker_repository_usages: Vec<DockerRepositoryBill>,
	domain_usages: HashMap<DomainPlan, DomainBill>,
	secret_usages: HashMap<u64, SecretsBill>,
	total_bill: f64,
	deployment_bill: f64,
	database_bill: f64,
	static_site_bill: f64,
	managed_url_bill: f64,
	docker_repo_bill: f64,
	managed_domain_bill: f64,
	managed_secret_bill: f64,
	month: String,
	year: i32,
) -> Result<(), Error> {
	send_email(
		InvoiceEmail {
			workspace_name,
			billing_address,
			deployment_usages,
			database_usages,
			static_site_usages,
			managed_url_usages,
			docker_repository_usages,
			domain_usages,
			secret_usages,
			total_bill,
			deployment_bill,
			database_bill,
			static_site_bill,
			managed_url_bill,
			docker_repo_bill,
			managed_domain_bill,
			managed_secret_bill,
			month,
			year,
		},
		email,
		None,
		"Patr invoice",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/bill-not-paid-delete-resources/template.json"]
struct UnpaidResourcesDeletedEmail {
	username: String,
	workspace_name: String,
	month: String,
	year: i32,
	total_bill: f64,
}

pub async fn send_unpaid_resources_deleted_email(
	email: Mailbox,
	username: String,
	workspace_name: String,
	month: String,
	year: i32,
	total_bill: f64,
) -> Result<(), Error> {
	send_email(
		UnpaidResourcesDeletedEmail {
			username,
			workspace_name,
			month,
			year,
			total_bill,
		},
		email,
		None,
		"[Action required] Patr resources deleted",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/bill-not-paid-reminder/template.json"]
struct BillNotPaidReminderEmail {
	username: String,
	workspace_name: String,
	month: String,
	year: i32,
	total_bill: f64,
}

pub async fn send_bill_not_paid_reminder_email(
	email: Mailbox,
	username: String,
	workspace_name: String,
	month: String,
	year: i32,
	total_bill: f64,
) -> Result<(), Error> {
	send_email(
		BillNotPaidReminderEmail {
			username,
			workspace_name,
			month,
			year,
			total_bill,
		},
		email,
		None,
		"[Action required] Patr bill payment pending",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/payment-failure/template.json"]
struct PaymentFailedEmail {
	username: String,
	workspace_name: String,
	month: String,
	year: i32,
	total_bill: f64,
}
pub async fn send_payment_failed_email(
	email: Mailbox,
	username: String,
	workspace_name: String,
	month: String,
	year: i32,
	total_bill: f64,
) -> Result<(), Error> {
	send_email(
		PaymentFailedEmail {
			username,
			workspace_name,
			month,
			year,
			total_bill,
		},
		email,
		None,
		"[Action required] Patr payment failed",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/deleted-resource/template.json"]
struct ResourceDeletedEmail {
	workspace_name: String,
	resource_name: String,
	username: String,
	deleted_by: String,
	message: String,
	resource_type: String,
}

pub async fn send_resource_deleted_email(
	workspace_name: String,
	resource_name: String,
	username: String,
	resource_type: String,
	deleted_by: String,
	message: String,
	email: Mailbox,
) -> Result<(), Error> {
	send_email(
		ResourceDeletedEmail {
			workspace_name,
			resource_name,
			username,
			deleted_by,
			message,
			resource_type,
		},
		email,
		None,
		"Patr resource deleted",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/domain-not-verified/template.json"]
struct DomainUnverified {
	domain: String,
	username: String,
}

pub async fn send_domain_unverified_email(
	domain: String,
	username: String,
	email: Mailbox,
) -> Result<(), Error> {
	send_email(
		DomainUnverified { domain, username },
		email,
		None,
		"Domain not Verified",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/domain-verified/template.json"]
struct DomainVerified {
	domain: String,
	username: String,
}

pub async fn send_domain_verified_email(
	domain: String,
	username: String,
	email: Mailbox,
) -> Result<(), Error> {
	send_email(
		DomainVerified { domain, username },
		email,
		None,
		"Domain Verified",
	)
	.await
}

// pub async fn send_resource_updated_email(
// 	resource_id: String,
// 	resource_name: String,
// 	super_admin_firstname: String,
// 	ip_address: String,
// 	city: String,
// 	region: String,
// 	country: String,
// 	email: String,
// ) -> Result<(), Error> {
// 	send_email(
// 		ResourceUpdatedEmail {
// 			resource_id,
// 			resource_name,
// 			super_admin_firstname,
// 			ip_address,
// 			city,
// 			region,
// 			country,
// 		},
// 		email,
// 		None,
// 		"Patr resource updated",
// 	)
// 	.await
// }
/// # Description
/// This function is used to send the email to a recipient
///
/// # Arguments
/// * `body` - body of the mail of the type [`TEmail`]
/// * `to` - recipient's email address of type [`Mailbox`]
/// * `reply_to` - An Option<Mailbox> containing instance of [`Mailbox`]
///   containing email of recipient
/// to be replied or `None`
/// * `subject` - a string containing subject of the email
///
/// # Returns
/// This function returns `Result<(), Error>` containing an empty response or an
/// errors
///
/// [`TEmail`]: TEmail
#[cfg(not(debug_assertions))]
async fn send_email<TEmail>(
	body: TEmail,
	to: Mailbox,
	reply_to: Option<Mailbox>,
	subject: &str,
) -> Result<(), Error>
where
	TEmail: EmailTemplate + Send + Sync + 'static,
{
	use lettre::{
		transport::smtp::authentication::Credentials,
		AsyncSmtpTransport,
		AsyncTransport,
		Message,
		Tokio1Executor,
	};
	use tokio::{task, task::JoinHandle};

	use crate::service;

	let subject = subject.to_string();
	let join_handle: JoinHandle<Result<_, Error>> = task::spawn(async move {
		let settings = service::get_settings();
		let mut builder = Message::builder()
			.from(settings.email.from.parse()?)
			.to(to.clone())
			.subject(subject);
		if let Some(reply_to) = reply_to {
			builder = builder.reply_to(reply_to);
		}

		let message = builder.multipart(body.render_body().await?)?;

		let credentials = Credentials::new(
			settings.email.username.clone(),
			settings.email.password.clone(),
		);

		let response = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(
			&settings.email.host,
		)?
		.credentials(credentials)
		.port(settings.email.port)
		.build::<Tokio1Executor>()
		.send(message)
		.await?;

		if !response.is_positive() {
			log::error!("Error sending email to `{}`: {}", to, response.code());
		}

		Ok(())
	});

	let _ = task::spawn(async {
		let result = join_handle.await;

		if let Ok(Err(error)) = result {
			// TODO log this error
			log::error!("Unable to send email: {}", error.get_error());
		}
	});

	Ok(())
}

#[cfg(debug_assertions)]
async fn send_email<TEmail>(
	_body: TEmail,
	to: Mailbox,
	_reply_to: Option<Mailbox>,
	_subject: &str,
) -> Result<(), Error>
where
	TEmail: EmailTemplate,
{
	log::trace!("Sending email to {}", to);
	Ok(())
}
