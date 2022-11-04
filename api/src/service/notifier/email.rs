use std::collections::BTreeMap;

use api_models::{
	models::workspace::billing::{
		DatabaseUsage,
		DeploymentUsage,
		DockerRepositoryUsage,
		DomainPlan,
		DomainUsage,
		ManagedUrlUsage,
		SecretUsage,
		StaticSitePlan,
		StaticSiteUsage,
		WorkspaceBillBreakdown, Address,
	},
	utils::{PriceAmount, Uuid},
};
use lettre::message::Mailbox;
use serde::Serialize;

use crate::{models::EmailTemplate, utils::Error};

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
#[allow(non_snake_case)]
struct RecoveryNotificationEmail {
	username: String,
	recoveryEmail: String,
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
#[allow(non_snake_case)]
pub async fn send_recovery_registration_mail(
	email: Mailbox,
	username: &str,
	recoveryEmail: &str,
) -> Result<(), Error> {
	send_email(
		RecoveryNotificationEmail {
			username: username.to_string(),
			recoveryEmail: recoveryEmail.to_string(),
		},
		email,
		None,
		"Welcome to Patr",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/add-new-email-notification/template.json"]
#[allow(non_snake_case)]
struct AddEmailVerificationEmail {
	otp: String,
	username: String,
	recoveryEmail: String,
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
	recovery_email: &str,
) -> Result<(), Error> {
	send_email(
		AddEmailVerificationEmail {
			username: username.to_string(),
			otp: otp.to_string(),
			recoveryEmail: recovery_email.to_string(),
		},
		email,
		None,
		"Patr email verification OTP",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/bill-not-paid-delete-resources/template.json"]
#[allow(non_snake_case)]
struct BillNotPaidDeleteResourcesEmail {
	username: String,
	workspaceName: String,
	monthString: String,
	monthNumber: u32,
	year: i32,
	totalBill: f64,
}

#[allow(non_snake_case)]
pub async fn send_bill_not_paid_delete_resources_email(
	email: Mailbox,
	username: String,
	workspaceName: String,
	monthString: String,
	monthNumber: u32,
	year: i32,
	totalBill: f64,
) -> Result<(), Error> {
	send_email(
		BillNotPaidDeleteResourcesEmail {
			username,
			workspaceName,
			monthString,
			monthNumber,
			year,
			totalBill,
		},
		email,
		None,
		"[Action required] Patr resources deleted",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/bill-payment-failed-reminder/template.json"]
#[allow(non_snake_case)]
struct BillPaymentFailedReminderEmail {
	username: String,
	workspaceName: String,
	monthString: String,
	monthNumber: u32,
	year: i32,
	totalBill: PriceAmount,
	deadline: String,
}

#[allow(non_snake_case)]
pub async fn send_bill_payment_failed_reminder_email(
	email: Mailbox,
	username: String,
	workspaceName: String,
	monthString: String,
	monthNumber: u32,
	year: i32,
	totalBill: PriceAmount,
	deadline: String,
) -> Result<(), Error> {
	send_email(
		BillPaymentFailedReminderEmail {
			username,
			workspaceName,
			monthString,
			monthNumber,
			year,
			totalBill,
			deadline,
		},
		email,
		None,
		"[Action required] Patr bill payment pending",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/card-not-added-reminder/template.json"]
#[allow(non_snake_case)]
struct CardNotAddedReminderEmail {
	username: String,
	workspaceName: String,
	monthString: String,
	monthNumber: u32,
	year: i32,
	totalBill: PriceAmount,
	deadline: String,
}

#[allow(non_snake_case)]
pub async fn send_card_not_added_reminder_email(
	email: Mailbox,
	username: String,
	workspaceName: String,
	monthString: String,
	monthNumber: u32,
	year: i32,
	totalBill: PriceAmount,
	deadline: String,
) -> Result<(), Error> {
	send_email(
		CardNotAddedReminderEmail {
			username,
			workspaceName,
			monthString,
			monthNumber,
			year,
			totalBill,
			deadline,
		},
		email,
		None,
		"[Action required] Patr bill payment pending",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/bill-paid-successfully/template.json"]
#[allow(non_snake_case)]
struct BillPaidSuccessfullyEmail {
	username: String,
	workspaceName: String,
	monthString: String,
	year: i32,
	cardAmountDeducted: PriceAmount,
}

#[allow(non_snake_case)]
pub async fn send_bill_paid_successfully_email(
	email: Mailbox,
	username: String,
	workspaceName: String,
	monthString: String,
	year: i32,
	cardAmountDeducted: PriceAmount,
) -> Result<(), Error> {
	send_email(
		BillPaidSuccessfullyEmail {
			username,
			workspaceName,
			monthString,
			year,
			cardAmountDeducted,
		},
		email,
		None,
		"Patr payment successful",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/payment-failure-invoice/template.json"]
#[allow(non_snake_case)]
struct PaymentFailureInvoiceEmail {
	username: String,
	workspaceName: String,
	billBreakdown: WorkspaceBillBreakdown,
	billingAddress: Address,
	monthString: String,
}

pub async fn send_payment_failure_invoice_email(
	email: Mailbox,
	username: String,
	workspace_name: String,
	bill_breakdown: WorkspaceBillBreakdown,
	billing_address: Address,
	month_string: String,
) -> Result<(), Error> {
	send_email(
		PaymentFailureInvoiceEmail {
			username,
			workspaceName: workspace_name,
			billBreakdown: bill_breakdown,
			billingAddress: billing_address,
			monthString: month_string,
		},
		email,
		None,
		"[Action required] Patr payment failed",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/payment-success-invoice/template.json"]
#[allow(non_snake_case)]
struct PaymentSuccessInvoiceEmail {
	username: String,
	workspaceName: String,
	billBreakdown: WorkspaceBillBreakdown,
	billingAddress: Address,
	monthString: String,
	creditsDeducted: PriceAmount,
	cardAmountDeducted: PriceAmount,
	creditsRemaining: PriceAmount,
}

pub async fn send_payment_success_invoice_email(
	email: Mailbox,
	username: String,
	workspace_name: String,
	bill_breakdown: WorkspaceBillBreakdown,
	billing_address: Address,
	month_string: String,
	credits_deducted: PriceAmount,
	card_amount_deducted: PriceAmount,
	credits_remaining: PriceAmount,
) -> Result<(), Error> {
	send_email(
		PaymentSuccessInvoiceEmail {
			username,
			workspaceName: workspace_name,
			billBreakdown: bill_breakdown,
			billingAddress: billing_address,
			monthString: month_string,
			creditsDeducted: credits_deducted,
			cardAmountDeducted: card_amount_deducted,
			creditsRemaining: credits_remaining,
		},
		email,
		None,
		"Patr payment successful",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/delete-resource/template.json"]
#[allow(non_snake_case)]
struct ResourceDeletedEmail {
	workspaceName: String,
	resourceName: String,
	username: String,
	deletedBy: String,
	resourceType: String,
}

#[allow(non_snake_case)]
pub async fn send_resource_deleted_email(
	workspaceName: String,
	resourceName: String,
	username: String,
	resourceType: String,
	deletedBy: String,
	email: Mailbox,
) -> Result<(), Error> {
	send_email(
		ResourceDeletedEmail {
			workspaceName,
			resourceName,
			username,
			deletedBy,
			resourceType,
		},
		email,
		None,
		"Patr resource deleted",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/domain-not-verified/template.json"]
#[allow(non_snake_case)]
struct DomainUnverified {
	domainName: String,
	domainId: String,
	username: String,
	isInternal: bool,
}

#[allow(non_snake_case)]
pub async fn send_domain_unverified_email(
	domainName: String,
	username: String,
	isInternal: bool,
	domainId: String,
	email: Mailbox,
) -> Result<(), Error> {
	send_email(
		DomainUnverified {
			domainName,
			username,
			isInternal,
			domainId,
		},
		email,
		None,
		"Domain not Verified",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/domain-verified/template.json"]
#[allow(non_snake_case)]
struct DomainVerified {
	domainName: String,
	username: String,
	domainId: String,
}

#[allow(non_snake_case)]
pub async fn send_domain_verified_email(
	domainName: String,
	username: String,
	domainId: String,
	email: Mailbox,
) -> Result<(), Error> {
	send_email(
		DomainVerified {
			domainName,
			username,
			domainId,
		},
		email,
		None,
		"Domain Verified",
	)
	.await
}

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
