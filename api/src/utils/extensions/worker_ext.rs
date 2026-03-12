use apalis::prelude::TaskSink;
use apalis_postgres::PostgresStorage;

use crate::{prelude::*, worker::WorkerTaskType};

/// Contains the extension trait to allow sending emails using the worker.
pub trait WorkerExt {
	/// Sends an email using the worker. This is a convenience method that
	/// allows you to send an email using the worker without having to interact
	/// with the worker directly. It takes in an [`Email`] struct, which
	/// contains the necessary information to send the email, such as the
	/// recipient and the type of email to be sent.
	fn send_email(
		&mut self,
		to: String,
		email: impl Into<EmailTemplateType>,
	) -> impl Future<Output = Result<(), ErrorType>>;
}

impl WorkerExt for PostgresStorage<WorkerTaskType> {
	async fn send_email(
		&mut self,
		to: String,
		email: impl Into<EmailTemplateType>,
	) -> Result<(), ErrorType> {
		self.push(WorkerTaskType::Email(Email {
			to,
			template: email.into(),
		}))
		.await
		.map_err(ErrorType::server_error)
	}
}
