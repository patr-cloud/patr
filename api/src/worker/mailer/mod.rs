use apalis::prelude::*;
use handlebars::Handlebars;
use rust_embed::RustEmbed;
use sendgrid::v3::{
	Attachment,
	Content,
	Disposition,
	Email as SGEmail,
	Message,
	Personalization,
	Sender,
};
use serde::{Deserialize, Serialize};

use crate::{app::WorkerTaskType, prelude::*};

/// The email that is sent when a user signs up, containing an OTP for
/// verification.
mod user_sign_up;

pub use self::user_sign_up::*;

/// The trait that all email templates must implement. This trait is used to
/// generate the necessary information to send an email, such as the subject,
/// the HTML and text filenames, and the inline attachments for the email.
pub trait EmailTemplate {
	/// The name of the email template. This is used to load the template data
	/// from the "template.json" file for the email template.
	fn template_name(&self) -> &'static str;

	/// The subject of the email.
	///
	/// This is generated from the "template.json" file for the email template.
	fn subject(&self) -> &'static str;

	/// The filename of the HTML content.
	///
	/// This is generated from the "template.json" file for the email template.
	fn html_file(&self) -> &'static str;

	/// The filename of the text content.
	///
	/// This is generated from the "template.json" file for the email template.
	fn text_file(&self) -> &'static str;

	/// The inline attachments for the email.
	///
	/// This is generated from the "template.json" file for the email template.
	fn inline_attachments(&self) -> Vec<&'static str>;
}

/// The struct representing an email to be sent by the worker. This struct
/// contains the necessary information to send an email, such as the recipient
/// and the type of email to be sent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
	/// The email address of the recipient.
	pub to: String,
	/// The type of email to be sent, containing the necessary data to send that
	/// email.
	#[serde(flatten)]
	pub r#type: EmailType,
}

/// The different types of emails that can be sent by the worker. Each variant
/// represents a different type of email, and contains the necessary data to
/// send that email.
///
/// Ideally you should never have to interact with this directly, and should
/// instead use the [`SendEmailExt`] extension trait on [`AppState`] to send
/// emails.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[doc(hidden)]
#[serde(tag = "type", content = "data")]
pub enum EmailType {
	/// An email sent to a user when they sign up, containing an OTP for
	/// verification.
	UserSignUp {
		/// The username of the user who signed up.
		username: String,
		/// The OTP to be sent to the user for verification.
		otp: String,
		/// The expiry time of the OTP, in a human-readable format.
		otp_expiry: String,
	},
}

/// The struct representing the email assets that are embedded in the binary.
/// This struct is used to access the email templates and their associated
/// assets, such as images, that are embedded in the binary using the
/// `rust_embed` crate.
#[derive(Debug, Clone, RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../assets/emails/"]
struct EmailAssets;

/// The function to send emails. This is used as a worker task, and is called by
/// the worker when an email needs to be sent. It takes in the email type, and
/// the app state, and sends the email accordingly.
pub(super) async fn send_emails(
	task: WorkerTaskType,
	state: Data<AppState>,
) -> Result<(), WorkerError> {
	let WorkerTaskType::Email(Email { to, r#type }) = task else {
		return Err(WorkerError::StateError(WorkerStateError::InvalidState(
			format!("Expected Email task, got {:?}", task),
		)));
	};

	// create handlebars instance and register the shared partials
	let mut handlebars = Handlebars::new();

	// register the shared partials
	EmailAssets::iter()
		.filter(|name| {
			if name.starts_with("shared/") {
				name.ends_with(".handlebars") || name.ends_with(".hbs")
			} else {
				false
			}
		})
		.try_for_each(|name| {
			let partial_file = EmailAssets::get(&name).ok_or_else(|| {
				WorkerStateError::InvalidState(format!("Partial file not found: `{}`", name))
			})?;
			let partial_str = String::from_utf8_lossy(partial_file.data.as_ref());

			handlebars
				.register_partial(
					name.trim_start_matches("shared/")
						.trim_end_matches(".handlebars")
						.trim_end_matches(".hbs"),
					partial_str,
				)
				.map_err(|err| {
					WorkerStateError::InvalidState(format!(
						"Failed to register partial `{}`: `{}`",
						name, err
					))
				})
		})?;

	let email = match r#type {
		EmailType::UserSignUp {
			username,
			otp,
			otp_expiry,
		} => UserSignUpEmail {
			username,
			otp,
			otp_expiry,
		},
	};

	let subject = email.subject();
	let html = EmailAssets::get(&format!("{}/{}", email.template_name(), email.html_file()))
		.ok_or_else(|| {
			WorkerStateError::InvalidState(format!(
				"HTML file not found for template `{}`: `{}`",
				email.template_name(),
				email.html_file()
			))
		})?
		.data;
	let html = String::from_utf8_lossy(&html);

	let text = EmailAssets::get(&dbg!(format!(
		"{}/{}",
		email.template_name(),
		email.text_file()
	)))
	.ok_or_else(|| {
		WorkerStateError::InvalidState(format!(
			"Text file not found for template `{}`: `{}`",
			email.template_name(),
			email.text_file()
		))
	})?
	.data;
	let text = String::from_utf8_lossy(&text);

	let subject = handlebars.render_template(subject, &email).map_err(|err| {
		WorkerStateError::InvalidState(format!("Failed to render subject template: `{}`", err))
	})?;
	let html = handlebars.render_template(&html, &email).map_err(|err| {
		WorkerStateError::InvalidState(format!("Failed to render HTML template: `{}`", err))
	})?;
	let text = handlebars.render_template(&text, &email).map_err(|err| {
		WorkerStateError::InvalidState(format!("Failed to render text template: `{}`", err))
	})?;

	let mut mail = Message::new(SGEmail::new(&state.config.email.from))
		.add_personalization(Personalization::new(SGEmail::new(&to)))
		.set_subject(&subject)
		.add_content(
			Content::new()
				.set_content_type("text/plain")
				.set_value(&text),
		)
		.add_content(
			Content::new()
				.set_content_type("text/html")
				.set_value(&html),
		);

	let attachments = email
		.inline_attachments()
		.iter()
		.map(|inline| {
			let attachment =
				EmailAssets::get(&format!("shared/images/{}", inline)).unwrap_or_else(|| {
					panic!(
						"Inline attachment not found for template `{}`: `{}`",
						email.template_name(),
						inline
					)
				});

			(
				attachment.data,
				inline.to_string(),
				attachment.metadata.mimetype().to_string(),
			)
		})
		.collect::<Vec<_>>();

	let attachments = attachments
		.iter()
		.map(|(data, inline, mimetype)| {
			Attachment::new()
				.set_content(data)
				.set_content_idm(&inline)
				.set_mime_type(&mimetype)
				.set_disposition(Disposition::Inline)
		})
		.collect::<Vec<_>>();

	for attachment in attachments {
		mail = mail.add_attachment(attachment);
	}

	Sender::new(&state.config.email.sendgrid_api_key, None)
		.send(&mail)
		.await
		.map_err(|err| {
			WorkerStateError::InvalidState(format!("Failed to send email: `{}`", err))
		})?;

	Ok(())
}
