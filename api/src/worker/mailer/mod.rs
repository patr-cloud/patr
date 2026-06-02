use std::fmt::Debug;

use apalis::prelude::*;
use derive_more::From;
use lettre::{
	AsyncSmtpTransport,
	AsyncTransport,
	Tokio1Executor,
	message::{MultiPart, header::ContentType},
	transport::smtp::authentication::Credentials,
};
use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// The add new email notification email template.
mod add_new_email_notification;
/// The backup email notification email template.
mod backup_email_notification;
/// The bill not paid delete resources email template.
mod bill_not_paid_delete_resources;
/// The bill paid successfully email template.
mod bill_paid_successfully;
/// The bill paid using credits email template.
mod bill_paid_using_credits;
/// The bill payment failed reminder email template.
mod bill_payment_failed_reminder;
/// The card not added reminder email template.
mod card_not_added_reminder;
/// The delete resource email template.
mod delete_resource;
/// The domain added email template.
mod domain_added;
/// The domain not verified email template.
mod domain_not_verified;
/// The domain not verified reminder email template.
mod domain_not_verified_reminder;
/// The domain verified email template.
mod domain_verified;
/// The forgot password email template.
mod forgot_password;
/// The partial payment success email template.
mod partial_payment_success;
/// The password changed notification email template.
mod password_changed_notification;
/// The password reset notification email template.
mod password_reset_notification;
/// The payment failure invoice email template.
mod payment_failure_invoice;
/// The payment success invoice email template.
mod payment_success_invoice;
/// The purchase credits success email template.
mod purchase_credits_success;
/// The recovery email notification email template.
mod recovery_email_notification;
/// The runner disconnected reminder email template.
mod runner_disconnected_reminder;
/// The sign-up completed email template.
mod sign_up_completed;
/// The unverified domain delete email template.
mod unverified_domain_delete;
/// The user sign-up email template.
mod user_sign_up;

pub use self::{
	add_new_email_notification::*,
	backup_email_notification::*,
	bill_not_paid_delete_resources::*,
	bill_paid_successfully::*,
	bill_paid_using_credits::*,
	bill_payment_failed_reminder::*,
	card_not_added_reminder::*,
	delete_resource::*,
	domain_added::*,
	domain_not_verified::*,
	domain_not_verified_reminder::*,
	domain_verified::*,
	forgot_password::*,
	partial_payment_success::*,
	password_changed_notification::*,
	password_reset_notification::*,
	payment_failure_invoice::*,
	payment_success_invoice::*,
	purchase_credits_success::*,
	recovery_email_notification::*,
	runner_disconnected_reminder::*,
	sign_up_completed::*,
	unverified_domain_delete::*,
	user_sign_up::*,
};

/// The struct representing an email to be sent by the worker. This struct
/// contains the necessary information to send an email, such as the recipient
/// and the type of email to be sent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Email {
	/// The email address of the recipient.
	pub to: String,
	/// The body of the email, which contains the necessary information to
	/// send the email template.
	#[serde(flatten)]
	pub template: EmailTemplateType,
}

/// Render-time context shared by every email template.
///
/// Constructed once per send in [`super::send_emails`] and passed to all
/// of an email template's render methods. Empty for now — fields will be
/// added here as templates need them.
#[derive(Debug, Clone, Default)]
pub struct GlobalArgs {}

/// The type of email to be sent. This is used to differentiate between
/// different types of email tasks, such as sending a verification email or
/// sending a password reset email. Each variant of this enum represents a
/// different type of email template, and contains the necessary information to
/// render that email template. The worker will use this information to render
/// the appropriate email template and send the email accordingly.
#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum EmailTemplateType {
	/// The add new email notification email template. This email is sent to
	/// the user when they add a new email address to their account.
	AddNewEmailNotification(add_new_email_notification::_AddNewEmailNotificationEmail),
	/// The backup email notification email template. This email is sent to
	/// the user when a recovery email is set for their account.
	BackupEmailNotification(backup_email_notification::_BackupEmailNotificationEmail),
	/// The bill not paid delete resources email template. This email is sent
	/// to the user when their resources have been deleted due to an unpaid
	/// balance.
	BillNotPaidDeleteResources(bill_not_paid_delete_resources::_BillNotPaidDeleteResourcesEmail),
	/// The bill paid successfully email template. This email is sent to the
	/// user when their payment has been processed successfully.
	BillPaidSuccessfully(bill_paid_successfully::_BillPaidSuccessfullyEmail),
	/// The bill paid using credits email template. This email is sent to the
	/// user when their bill has been fully or partially paid using Patr
	/// credits.
	BillPaidUsingCredits(bill_paid_using_credits::_BillPaidUsingCreditsEmail),
	/// The bill payment failed reminder email template. This email is sent to
	/// the user when their bill is unpaid and resources are at risk of
	/// deletion.
	BillPaymentFailedReminder(bill_payment_failed_reminder::_BillPaymentFailedReminderEmail),
	/// The card not added reminder email template. This email is sent to
	/// remind the user to add a payment method to avoid resource deletion.
	CardNotAddedReminder(card_not_added_reminder::_CardNotAddedReminderEmail),
	/// The delete resource email template. This email is sent when a resource
	/// is deleted from a workspace.
	DeleteResource(delete_resource::_DeleteResourceEmail),
	/// The forgot password email template. This email is sent to the user
	/// when they request a password reset.
	ForgotPassword(forgot_password::ForgotPasswordEmail),
	/// The domain added email template. This email is sent to the user when
	/// a domain is added to their workspace, prompting DNS verification.
	DomainAdded(domain_added::_DomainAddedEmail),
	/// The domain not verified email template. This email is sent to the user
	/// when their domain is no longer pointing to Patr.
	DomainNotVerified(domain_not_verified::_DomainNotVerifiedEmail),
	/// The domain not verified reminder email template. This email is sent to
	/// the user when a domain in their workspace has not been verified yet.
	DomainNotVerifiedReminder(domain_not_verified_reminder::_DomainNotVerifiedReminderEmail),
	/// The domain verified email template. This email is sent to the
	/// user when their domain has been successfully verified.
	DomainVerified(domain_verified::_DomainVerifiedEmail),
	/// The partial payment success email template. This email is sent to the
	/// user when a partial payment has been received toward their bill.
	PartialPaymentSuccess(partial_payment_success::_PartialPaymentSuccessEmail),
	/// The password changed notification email template. This email is sent
	/// to the user when their password has been changed.
	PasswordChangedNotification(password_changed_notification::_PasswordChangedNotificationEmail),
	/// The password reset notification email template. This email is sent to
	/// the user when their password has been reset.
	PasswordResetNotification(password_reset_notification::_PasswordResetNotificationEmail),
	/// The payment failure invoice email template. This email is sent to the
	/// user when a payment fails to process for their workspace.
	PaymentFailureInvoice(payment_failure_invoice::_PaymentFailureInvoiceEmail),
	/// The payment success invoice email template. This email is sent to the
	/// user when their payment has been successfully processed for a workspace.
	PaymentSuccessInvoice(payment_success_invoice::_PaymentSuccessInvoiceEmail),
	/// The purchase credits success email template. This email is sent to the
	/// user when credits have been successfully added to their workspace.
	PurchaseCreditsSuccess(purchase_credits_success::_PurchaseCreditsSuccessEmail),
	/// The recovery email notification email template. This email is sent when
	/// a recovery email address is added to a user's Patr account.
	RecoveryEmailNotification(recovery_email_notification::_RecoveryEmailNotificationEmail),
	/// The runner disconnected reminder email template. This email is sent to
	/// the user when a runner in their workspace is no longer connected.
	RunnerDisconnectedReminder(runner_disconnected_reminder::_RunnerDisconnectedReminderEmail),
	/// The sign-up completed email template. This email is sent to the user
	/// when their account setup is complete.
	SignUpCompleted(sign_up_completed::SignUpCompletedEmail),
	/// The unverified domain delete email template. This email is sent to the
	/// user when their domain is removed due to not being verified in time.
	UnverifiedDomainDelete(unverified_domain_delete::_UnverifiedDomainDeleteEmail),
	/// The user sign-up email template. This email is sent to the user when
	/// they sign up, and contains an OTP for verification.
	UserSignUp(user_sign_up::UserSignUpEmail),
}

impl EmailTemplateType {
	/// Renders the subject template of the email into a string.
	fn render_subject(&self, globals: &GlobalArgs) -> Result<String, ErrorType> {
		match self {
			Self::AddNewEmailNotification(template) => template.render_subject(globals),
			Self::BackupEmailNotification(template) => template.render_subject(globals),
			Self::BillNotPaidDeleteResources(template) => template.render_subject(globals),
			Self::BillPaidSuccessfully(template) => template.render_subject(globals),
			Self::BillPaidUsingCredits(template) => template.render_subject(globals),
			Self::BillPaymentFailedReminder(template) => template.render_subject(globals),
			Self::CardNotAddedReminder(template) => template.render_subject(globals),
			Self::DeleteResource(template) => template.render_subject(globals),
			Self::ForgotPassword(template) => template.render_subject(globals),
			Self::DomainAdded(template) => template.render_subject(globals),
			Self::DomainNotVerified(template) => template.render_subject(globals),
			Self::DomainNotVerifiedReminder(template) => template.render_subject(globals),
			Self::DomainVerified(template) => template.render_subject(globals),
			Self::PartialPaymentSuccess(template) => template.render_subject(globals),
			Self::PasswordChangedNotification(template) => template.render_subject(globals),
			Self::PasswordResetNotification(template) => template.render_subject(globals),
			Self::PaymentFailureInvoice(template) => template.render_subject(globals),
			Self::PaymentSuccessInvoice(template) => template.render_subject(globals),
			Self::PurchaseCreditsSuccess(template) => template.render_subject(globals),
			Self::RecoveryEmailNotification(template) => template.render_subject(globals),
			Self::RunnerDisconnectedReminder(template) => template.render_subject(globals),
			Self::SignUpCompleted(template) => template.render_subject(globals),
			Self::UnverifiedDomainDelete(template) => template.render_subject(globals),
			Self::UserSignUp(template) => template.render_subject(globals),
		}
	}

	fn render_html(&self, globals: &GlobalArgs) -> Result<String, ErrorType> {
		match self {
			Self::AddNewEmailNotification(template) => template.render_html(globals),
			Self::BackupEmailNotification(template) => template.render_html(globals),
			Self::BillNotPaidDeleteResources(template) => template.render_html(globals),
			Self::BillPaidSuccessfully(template) => template.render_html(globals),
			Self::BillPaidUsingCredits(template) => template.render_html(globals),
			Self::BillPaymentFailedReminder(template) => template.render_html(globals),
			Self::CardNotAddedReminder(template) => template.render_html(globals),
			Self::DeleteResource(template) => template.render_html(globals),
			Self::ForgotPassword(template) => template.render_html(globals),
			Self::DomainAdded(template) => template.render_html(globals),
			Self::DomainNotVerified(template) => template.render_html(globals),
			Self::DomainNotVerifiedReminder(template) => template.render_html(globals),
			Self::DomainVerified(template) => template.render_html(globals),
			Self::PartialPaymentSuccess(template) => template.render_html(globals),
			Self::PasswordChangedNotification(template) => template.render_html(globals),
			Self::PasswordResetNotification(template) => template.render_html(globals),
			Self::PaymentFailureInvoice(template) => template.render_html(globals),
			Self::PaymentSuccessInvoice(template) => template.render_html(globals),
			Self::PurchaseCreditsSuccess(template) => template.render_html(globals),
			Self::RecoveryEmailNotification(template) => template.render_html(globals),
			Self::RunnerDisconnectedReminder(template) => template.render_html(globals),
			Self::SignUpCompleted(template) => template.render_html(globals),
			Self::UnverifiedDomainDelete(template) => template.render_html(globals),
			Self::UserSignUp(template) => template.render_html(globals),
		}
	}

	fn render_text(&self, globals: &GlobalArgs) -> Result<String, ErrorType> {
		match self {
			Self::AddNewEmailNotification(template) => template.render_text(globals),
			Self::BackupEmailNotification(template) => template.render_text(globals),
			Self::BillNotPaidDeleteResources(template) => template.render_text(globals),
			Self::BillPaidSuccessfully(template) => template.render_text(globals),
			Self::BillPaidUsingCredits(template) => template.render_text(globals),
			Self::BillPaymentFailedReminder(template) => template.render_text(globals),
			Self::CardNotAddedReminder(template) => template.render_text(globals),
			Self::DeleteResource(template) => template.render_text(globals),
			Self::ForgotPassword(template) => template.render_text(globals),
			Self::DomainAdded(template) => template.render_text(globals),
			Self::DomainNotVerified(template) => template.render_text(globals),
			Self::DomainNotVerifiedReminder(template) => template.render_text(globals),
			Self::DomainVerified(template) => template.render_text(globals),
			Self::PartialPaymentSuccess(template) => template.render_text(globals),
			Self::PasswordChangedNotification(template) => template.render_text(globals),
			Self::PasswordResetNotification(template) => template.render_text(globals),
			Self::PaymentFailureInvoice(template) => template.render_text(globals),
			Self::PaymentSuccessInvoice(template) => template.render_text(globals),
			Self::PurchaseCreditsSuccess(template) => template.render_text(globals),
			Self::RecoveryEmailNotification(template) => template.render_text(globals),
			Self::RunnerDisconnectedReminder(template) => template.render_text(globals),
			Self::SignUpCompleted(template) => template.render_text(globals),
			Self::UnverifiedDomainDelete(template) => template.render_text(globals),
			Self::UserSignUp(template) => template.render_text(globals),
		}
	}
}

/// The function to send emails. This is used as a worker task, and is called by
/// the worker when an email needs to be sent. It takes in the email type, and
/// the app state, and sends the email accordingly.
pub(super) async fn send_emails(email: Email, state: Data<AppState>) -> Result<(), WorkerError> {
	let globals = GlobalArgs::default();
	let subject = email.template.render_subject(&globals).map_err(|e| {
		WorkerError::StateError(WorkerStateError::InvalidState(format!(
			"Failed to render email template: {e}"
		)))
	})?;
	let html = email.template.render_html(&globals).map_err(|e| {
		WorkerError::StateError(WorkerStateError::InvalidState(format!(
			"Failed to render email template: {e}"
		)))
	})?;
	let text = email.template.render_text(&globals).map_err(|e| {
		WorkerError::StateError(WorkerStateError::InvalidState(format!(
			"Failed to render email template: {e}"
		)))
	})?;

	let message = lettre::Message::builder()
		.from(state.config.email.from.parse().map_err(|e| {
			WorkerError::StateError(WorkerStateError::InvalidState(format!(
				"Invalid from address: {e}"
			)))
		})?)
		.to(email.to.parse().map_err(|e| {
			WorkerError::StateError(WorkerStateError::InvalidState(format!(
				"Invalid to address: {e}"
			)))
		})?)
		.subject(subject)
		.multipart(
			MultiPart::alternative()
				.singlepart(
					lettre::message::SinglePart::builder()
						.header(ContentType::TEXT_PLAIN)
						.body(text),
				)
				.singlepart(
					lettre::message::SinglePart::builder()
						.header(ContentType::TEXT_HTML)
						.body(html),
				),
		)
		.map_err(|e| {
			WorkerError::StateError(WorkerStateError::InvalidState(format!(
				"Failed to build email message: {e}"
			)))
		})?;

	if state.config.email.secure {
		AsyncSmtpTransport::<Tokio1Executor>::relay(&state.config.email.host)
	} else {
		AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&state.config.email.host)
	}
	.map_err(|e| {
		WorkerError::StateError(WorkerStateError::InvalidState(format!(
			"Failed to create SMTP transport: {e}"
		)))
	})?
	.port(state.config.email.port)
	.credentials(Credentials::new(
		state.config.email.username.clone(),
		state.config.email.password.clone(),
	))
	.build()
	.send(message)
	.await
	.map_err(|e| {
		WorkerError::StateError(WorkerStateError::InvalidState(format!(
			"Failed to send email: {e}"
		)))
	})?;

	info!("Email sent successfully to {}", email.to);

	Ok(())
}
