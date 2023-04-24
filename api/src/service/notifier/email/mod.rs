#[cfg(test)]
mod tests;

use api_models::models::workspace::billing::{Address, WorkspaceBillBreakdown};
use lettre::message::Mailbox;
use serde::Serialize;

use crate::{
	models::{EmailTemplate, UserDeployment},
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
		"Patr password changed successfully",
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
		"Patr password changed successfully",
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
		"Welcome to Patr!",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/recovery-email-notification/template.json"]
#[serde(rename_all = "camelCase")]
struct RecoveryNotificationEmail {
	username: String,
	recovery_email: String,
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
	recovery_email: &str,
) -> Result<(), Error> {
	send_email(
		RecoveryNotificationEmail {
			username: username.to_string(),
			recovery_email: recovery_email.to_string(),
		},
		email,
		None,
		"Recovery email added successfully",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/add-new-email-notification/template.json"]
#[serde(rename_all = "camelCase")]
struct AddEmailVerificationEmail {
	otp: String,
	username: String,
	recovery_email: String,
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
			recovery_email: recovery_email.to_string(),
		},
		email,
		None,
		"Patr email verification OTP",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/bill-not-paid-delete-resources/template.json"]
#[serde(rename_all = "camelCase")]
struct BillNotPaidDeleteResourcesEmail {
	username: String,
	workspace_name: String,
	month: u32,
	year: i32,
	total_bill: u64,
}

pub async fn send_bill_not_paid_delete_resources_email(
	email: Mailbox,
	username: String,
	workspace_name: String,
	month: u32,
	year: i32,
	total_bill: u64,
) -> Result<(), Error> {
	send_email(
		BillNotPaidDeleteResourcesEmail {
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
#[template_path = "assets/emails/bill-payment-failed-reminder/template.json"]
#[serde(rename_all = "camelCase")]
struct BillPaymentFailedReminderEmail {
	username: String,
	workspace_name: String,
	month: u32,
	year: i32,
	total_bill: u64,
	deadline: String,
}

pub async fn send_bill_payment_failed_reminder_email(
	email: Mailbox,
	username: String,
	workspace_name: String,
	month: u32,
	year: i32,
	total_bill: u64,
	deadline: String,
) -> Result<(), Error> {
	send_email(
		BillPaymentFailedReminderEmail {
			username,
			workspace_name,
			month,
			year,
			total_bill,
			deadline,
		},
		email,
		None,
		"[Action required] Patr bill payment failed",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/card-not-added-reminder/template.json"]
#[serde(rename_all = "camelCase")]
struct CardNotAddedReminderEmail {
	username: String,
	workspace_name: String,
	month: u32,
	year: i32,
	total_bill: u64,
	deadline: String,
}

pub async fn send_card_not_added_reminder_email(
	email: Mailbox,
	username: String,
	workspace_name: String,
	month: u32,
	year: i32,
	total_bill: u64,
	deadline: String,
) -> Result<(), Error> {
	send_email(
		CardNotAddedReminderEmail {
			username,
			workspace_name,
			month,
			year,
			total_bill,
			deadline,
		},
		email,
		None,
		"[Action required] Add payment method on Patr",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/bill-paid-successfully/template.json"]
#[serde(rename_all = "camelCase")]
struct BillPaidSuccessfullyEmail {
	username: String,
	workspace_name: String,
	month: u32,
	year: i32,
	card_amount_deducted: u64,
}

pub async fn send_bill_paid_successfully_email(
	email: Mailbox,
	username: String,
	workspace_name: String,
	month: u32,
	year: i32,
	card_amount_deducted: u64,
) -> Result<(), Error> {
	send_email(
		BillPaidSuccessfullyEmail {
			username,
			workspace_name,
			month,
			year,
			card_amount_deducted,
		},
		email,
		None,
		"Patr payment successful",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/payment-failure-invoice/template.json"]
#[serde(rename_all = "camelCase")]
struct PaymentFailureInvoiceEmail {
	username: String,
	workspace_name: String,
	bill_breakdown: WorkspaceBillBreakdown,
	billing_address: Address,
}

pub async fn send_payment_failure_invoice_email(
	email: Mailbox,
	username: String,
	workspace_name: String,
	bill_breakdown: WorkspaceBillBreakdown,
	billing_address: Address,
) -> Result<(), Error> {
	send_email(
		PaymentFailureInvoiceEmail {
			username,
			workspace_name,
			bill_breakdown,
			billing_address,
		},
		email,
		None,
		"[Action required] Patr payment failed",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/payment-success-invoice/template.json"]
#[serde(rename_all = "camelCase")]
struct PaymentSuccessInvoiceEmail {
	username: String,
	workspace_name: String,
	bill_breakdown: WorkspaceBillBreakdown,
	billing_address: Address,
	credits_deducted: u64,
	card_amount_deducted: u64,
	credits_remaining: u64,
}

pub async fn send_payment_success_invoice_email(
	email: Mailbox,
	username: String,
	workspace_name: String,
	bill_breakdown: WorkspaceBillBreakdown,
	billing_address: Address,
	credits_deducted: u64,
	card_amount_deducted: u64,
	credits_remaining: u64,
) -> Result<(), Error> {
	let has_bill = bill_breakdown.total_charge > 0;
	let is_charged = card_amount_deducted > 0;
	send_email(
		PaymentSuccessInvoiceEmail {
			username,
			workspace_name,
			bill_breakdown,
			billing_address,
			credits_deducted,
			card_amount_deducted,
			credits_remaining,
		},
		email,
		None,
		match (has_bill, is_charged) {
			(true, true) => "Patr payment successful via card",
			(true, false) => "Patr payment successful via credits",
			(false, _) => "Patr invoice",
		},
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/delete-resource/template.json"]
#[serde(rename_all = "camelCase")]
struct ResourceDeletedEmail {
	workspace_name: String,
	resource_name: String,
	username: String,
	deleted_by: String,
	resource_type: String,
}

pub async fn send_resource_deleted_email(
	workspace_name: String,
	resource_name: String,
	username: String,
	resource_type: String,
	deleted_by: String,
	email: Mailbox,
) -> Result<(), Error> {
	send_email(
		ResourceDeletedEmail {
			workspace_name,
			resource_name,
			username,
			deleted_by,
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
#[serde(rename_all = "camelCase")]
struct DomainUnverified {
	domain_name: String,
	domain_id: String,
	username: String,
	is_internal: bool,
	deadline_limit: u64,
}

pub async fn send_domain_unverified_email(
	domain_name: String,
	username: String,
	is_internal: bool,
	domain_id: String,
	deadline_limit: u64,
	email: Mailbox,
) -> Result<(), Error> {
	send_email(
		DomainUnverified {
			domain_name,
			username,
			is_internal,
			domain_id,
			deadline_limit,
		},
		email,
		None,
		"[Action Required] Domain not Verified",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/domain-not-verified-reminder/template.json"]
#[serde(rename_all = "camelCase")]
struct DomainVerificationReminder {
	domain_name: String,
	domain_id: String,
	username: String,
	is_internal: bool,
	patr_nameservers1: String,
	patr_nameservers2: String,
	patr_verify_sub_domain: String,
	deadline_limit: u64,
}

pub async fn send_domain_verify_reminder_email(
	domain_name: String,
	username: String,
	is_internal: bool,
	domain_id: String,
	patr_nameservers1: String,
	patr_nameservers2: String,
	patr_verify_sub_domain: String,
	deadline_limit: u64,
	email: Mailbox,
) -> Result<(), Error> {
	send_email(
		DomainVerificationReminder {
			domain_name,
			username,
			is_internal,
			domain_id,
			patr_nameservers1,
			patr_nameservers2,
			patr_verify_sub_domain,
			deadline_limit,
		},
		email,
		None,
		"[Action required] Domain not verified",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/domain-verified/template.json"]
#[serde(rename_all = "camelCase")]
struct DomainVerified {
	domain_name: String,
	username: String,
	domain_id: String,
}

pub async fn send_domain_verified_email(
	domain_name: String,
	username: String,
	domain_id: String,
	email: Mailbox,
) -> Result<(), Error> {
	send_email(
		DomainVerified {
			domain_name,
			username,
			domain_id,
		},
		email,
		None,
		"Domain Verified on Patr",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/byoc-disconnected-reminder/template.json"]
#[serde(rename_all = "camelCase")]
struct ByocDisconnectedReminder {
	username: String,
	workspace_name: String,
	region_name: String,
	region_id: String,
	deadline_limit: u64,
}

pub async fn send_byoc_disconnected_reminder_email(
	username: String,
	workspace_name: String,
	region_name: String,
	region_id: String,
	deadline_limit: u64,
	email: Mailbox,
) -> Result<(), Error> {
	send_email(
		ByocDisconnectedReminder {
			username,
			workspace_name,
			region_name,
			region_id,
			deadline_limit,
		},
		email,
		None,
		"[Action required] BYOC cluster not reachable",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/repo-storage-limit-exceed/template.json"]
#[serde(rename_all = "camelCase")]
struct RepositoryStorageLimitExceedEmail {
	username: String,
	workspace_name: String,
	repository_name: String,
	tag: String,
	digest: String,
	ip_address: String,
}

pub async fn send_repository_storage_limit_exceed_email(
	email: Mailbox,
	username: &str,
	workspace_name: &str,
	repository_name: &str,
	tag: &str,
	digest: &str,
	ip_address: &str,
) -> Result<(), Error> {
	send_email(
		RepositoryStorageLimitExceedEmail {
			username: username.to_owned(),
			workspace_name: workspace_name.to_owned(),
			repository_name: repository_name.to_owned(),
			tag: tag.to_owned(),
			digest: digest.to_owned(),
			ip_address: ip_address.to_owned(),
		},
		email,
		None,
		"[Action Required] Patr Repository Storage Limit Exceeded",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/purchase-credits-success/template.json"]
#[serde(rename_all = "camelCase")]
struct PurchaseCreditsSuccessEmail {
	username: String,
	workspace_name: String,
	credits_purchased: u64,
}

pub async fn send_purchase_credits_success_email(
	email: Mailbox,
	username: &str,
	workspace_name: &str,
	credits_purchased: u64,
) -> Result<(), Error> {
	send_email(
		PurchaseCreditsSuccessEmail {
			username: username.to_owned(),
			workspace_name: workspace_name.to_owned(),
			credits_purchased,
		},
		email,
		None,
		"Patr credits purchase successful",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/bill-paid-using-credits/template.json"]
#[serde(rename_all = "camelCase")]
struct BillPaidUsingCreditsEmail {
	username: String,
	workspace_name: String,
	total_bill: u64,
	bill_remaining: u64,
	credits_remaining: u64,
}

pub async fn send_bill_paid_using_credits_email(
	email: Mailbox,
	username: &str,
	workspace_name: &str,
	total_bill: u64,
	bill_remaining: u64,
	credits_remaining: u64,
) -> Result<(), Error> {
	send_email(
		BillPaidUsingCreditsEmail {
			username: username.to_owned(),
			workspace_name: workspace_name.to_owned(),
			total_bill,
			bill_remaining,
			credits_remaining,
		},
		email,
		None,
		"Patr payment successful",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/partial-payment-success/template.json"]
#[serde(rename_all = "camelCase")]
struct PartialPaymentSuccessEmail {
	username: String,
	workspace_name: String,
	total_bill: u64,
	amount_paid: u64,
	bill_remaining: u64,
	credits_remaining: u64,
}

pub async fn send_partial_payment_success_email(
	email: Mailbox,
	username: &str,
	workspace_name: &str,
	total_bill: u64,
	amount_paid: u64,
	bill_remaining: u64,
	credits_remaining: u64,
) -> Result<(), Error> {
	send_email(
		PartialPaymentSuccessEmail {
			username: username.to_owned(),
			workspace_name: workspace_name.to_owned(),
			total_bill,
			amount_paid,
			bill_remaining,
			credits_remaining,
		},
		email,
		None,
		"Patr payment successful",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/report-card/template.json"]
#[serde(rename_all = "camelCase")]
struct ReportCardEmail {
	username: String,
	resource_type: String,
	user_deployment: Vec<UserDeployment>,
}

pub async fn send_report_card_email(
	email: Mailbox,
	username: &str,
	resource_type: &str,
	user_deployment: &Vec<UserDeployment>,
) -> Result<(), Error> {
	send_email(
		ReportCardEmail {
			username: username.to_string(),
			resource_type: resource_type.to_string(),
			user_deployment: user_deployment.clone(),
		},
		email,
		None,
		"Patr: Report Card",
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

	use crate::{service, utils::handlebar_registry};

	let subject = subject.to_string();
	let join_handle: JoinHandle<Result<_, Error>> = task::spawn(async move {
		let settings = service::get_settings();
		let mut builder = Message::builder()
			.from(settings.email.from.parse()?)
			.to(to.clone())
			.subject(subject);
		if let Some(reply_to) = reply_to {
			builder = builder.reply_to(reply_to);
		} else {
			builder = builder.reply_to("support@patr.cloud".parse()?);
		}

		let message = builder.multipart(
			body.render_body(handlebar_registry::get_handlebar_registry())
				.await?,
		)?;

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

	task::spawn(async {
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
	body: TEmail,
	to: Mailbox,
	_reply_to: Option<Mailbox>,
	_subject: &str,
) -> Result<(), Error>
where
	TEmail: EmailTemplate,
{
	use crate::utils::handlebar_registry;

	log::trace!("Sending email to {}", to);

	let handlebar = handlebar_registry::get_handlebar_registry();
	body.render_body(handlebar).await.expect(
		// safe to panic as it is will be used only in debug builds
		"Handlebar template should be up-to-date with struct changes",
	);

	Ok(())
}
