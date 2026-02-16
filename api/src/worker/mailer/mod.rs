use apalis::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{app::WorkerTaskType, prelude::*};

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
	pub r#type: EmailTypeData,
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
#[serde(tag = "__emailType", rename_all = "camelCase")]
pub enum EmailTypeData {
	/// An email sent to a user when they sign up, containing an OTP for
	/// verification.
	#[serde(rename_all = "camelCase")]
	UserSignUp {
		/// The username of the user who signed up.
		username: String,
		/// The OTP to be sent to the user for verification.
		otp: String,
		/// The expiry time of the OTP, in a human-readable format.
		otp_expiry: String,
	},
	/// An email sent to a user when they complete the sign-up process,
	/// confirming that their account has been successfully created.
	#[serde(rename_all = "camelCase")]
	SignUpCompleted { username: String },
}

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

	// TODO send the actual email

	Ok(())
}
