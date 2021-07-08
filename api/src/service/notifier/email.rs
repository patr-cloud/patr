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
		let settings = service::get_config();
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
