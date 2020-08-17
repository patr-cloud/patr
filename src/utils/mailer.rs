use crate::utils::settings::Settings;

use lettre::{
	header,
	message::SinglePart,
	transport::smtp::authentication::Credentials,
	Message,
	SmtpTransport,
	Transport,
};

pub fn send_email_verification_mail(
	config: Settings,
	to_email: String,
	verification_token: String,
) {
	let from = format!("Bytesonus <{}>", config.email.from)
		.parse()
		.unwrap();
	let to = to_email.parse().unwrap();

	let email = Message::builder()
		.from(from)
		.to(to)
		.subject("Verify your email")
		.singlepart(
			SinglePart::eight_bit()
				.header(header::ContentType(
					"text/html; charset=utf8".parse().unwrap(),
				))
				.body(format!(
					"<p><b>Hello</b>, <i>world</i>!<br>Here's your token: {}</p>",
					verification_token
				)),
		)
		.unwrap();

	// Open a remote connection to gmail
	let mailer = SmtpTransport::relay(&config.email.host)
		.unwrap()
		.credentials(Credentials::new(
			config.email.username,
			config.email.password,
		))
		.build();

	// Send the email
	match mailer.send(&email) {
		Ok(_) => {
			log::info!(target: "emails", "Verification email to {} sent successfully!", to_email);
		}
		Err(err) => {
			log::error!(target: "emails", "Could not send email to {}: {:#?}", to_email, err);
		}
	}
}
