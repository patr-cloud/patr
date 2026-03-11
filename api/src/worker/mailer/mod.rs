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

/// The module to handle embedded email images. This is used to upload all
/// embedded email images to R2/S3 at startup.
pub mod images;

/// The user sign-up email template.
mod user_sign_up;

pub use self::images::upload_email_images;

/// Helper module for all emails.
pub mod mails {
	pub use super::user_sign_up::UserSignUpEmail;
}

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

/// The type of email to be sent. This is used to differentiate between
/// different types of email tasks, such as sending a verification email or
/// sending a password reset email. Each variant of this enum represents a
/// different type of email template, and contains the necessary information to
/// render that email template. The worker will use this information to render
/// the appropriate email template and send the email accordingly.
#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum EmailTemplateType {
	/// The user sign-up email template. This email is sent to the user when
	/// they sign up, and contains an OTP for verification.
	UserSignUp(user_sign_up::UserSignUpEmail),
}

impl EmailTemplateType {
	/// Renders the subject template of the email into a string.
	fn render_subject(&self) -> Result<String, ErrorType> {
		match self {
			EmailTemplateType::UserSignUp(template) => template.render_subject(),
		}
	}

	fn render_html(&self) -> Result<String, ErrorType> {
		match self {
			EmailTemplateType::UserSignUp(template) => template.render_html(),
		}
	}

	fn render_text(&self) -> Result<String, ErrorType> {
		match self {
			EmailTemplateType::UserSignUp(template) => template.render_text(),
		}
	}
}

/// The function to send emails. This is used as a worker task, and is called by
/// the worker when an email needs to be sent. It takes in the email type, and
/// the app state, and sends the email accordingly.
pub(super) async fn send_emails(email: Email, state: Data<AppState>) -> Result<(), WorkerError> {
	let subject = email.template.render_subject().map_err(|e| {
		WorkerError::StateError(WorkerStateError::InvalidState(format!(
			"Failed to render email template: {e}"
		)))
	})?;
	let html = email.template.render_html().map_err(|e| {
		WorkerError::StateError(WorkerStateError::InvalidState(format!(
			"Failed to render email template: {e}"
		)))
	})?;
	let text = email.template.render_text().map_err(|e| {
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
