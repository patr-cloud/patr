use lettre::message::Mailbox;
use serde::Serialize;

use crate::{models::EmailTemplate, utils::Error};

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/user-sign-up/template.json"]
struct UserSignUpVerificationEmail {
	otp: String,
}

pub async fn send_user_verification_otp(
	email: Mailbox,
	otp: &str,
) -> Result<(), Error> {
	send_email(
		UserSignUpVerificationEmail {
			otp: otp.to_string(),
		},
		email,
		None,
		"",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/forgot-password/template.json"]
struct ForgotPasswordEmail {
	otp: String,
}

pub async fn send_forgot_password_otp(
	email: Mailbox,
	otp: &str,
) -> Result<(), Error> {
	send_email(
		ForgotPasswordEmail {
			otp: otp.to_string(),
		},
		email,
		None,
		"",
	)
	.await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/password-reset-notification/template.json"]
struct PasswordResetEmail {}

pub async fn send_user_reset_password_notification(
	email: Mailbox,
) -> Result<(), Error> {
	send_email(PasswordResetEmail {}, email, None, "").await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/password-changed-notification/template.json"]
struct PasswordChangedEmail {}

pub async fn send_password_changed_notification(
	email: Mailbox,
) -> Result<(), Error> {
	send_email(PasswordChangedEmail {}, email, None, "").await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/sign-up-completed/template.json"]
struct SignUpCompletedEmail {}

pub async fn send_sign_up_completed_email(email: Mailbox) -> Result<(), Error> {
	send_email(SignUpCompletedEmail {}, email, None, "").await
}

#[derive(EmailTemplate, Serialize)]
#[template_path = "assets/emails/backup-email-notification/template.json"]
struct BackupNotificationEmail {}

pub async fn send_backup_registration_mail(
	email: Mailbox,
) -> Result<(), Error> {
	send_email(BackupNotificationEmail {}, email, None, "").await
}

#[cfg(not(debug_assertions))]
async fn send_email<TEmail>(
	body: TEmail,
	to: Mailbox,
	reply_to: Option<Mailbox>,
	subject: &str,
) -> Result<(), Error>
where
	TEmail: EmailTemplate,
{
	use lettre::{
		transport::smtp::authentication::Credentials,
		AsyncSmtpTransport,
		AsyncTransport,
		Message,
		Tokio1Executor,
	};

	use crate::service;

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

	let send_result = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(
		&settings.email.host,
	)?
	.credentials(credentials)
	.port(settings.email.port)
	.build::<Tokio1Executor>()
	.send(message)
	.await;

	if let Err(error) = send_result {
		// TODO log this error
		log::error!("Unable to send email to `{}`: {}", to, error);
	}

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
