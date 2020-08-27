use crate::utils::settings::Settings;

use lettre::{
	header,
	message::{MultiPart, SinglePart},
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
		.multipart(
			MultiPart::alternative()
				.singlepart(
					SinglePart::base64()
						.header(header::ContentType(
							"text/html; charset=utf8".parse().unwrap(),
						))
						.body(format!(
							r#"Verify your account 4 pizza. <br>
						Here's a simple link to use for the token:
							<a href="https://accounts.vicara.co/auth/verify?token={token}&email={email}">
								https://accounts.vicara.co/auth/verify?token={token}&email={email}
							</a>
						<br>
						This token expires in 24 hours.<br>
						TODO: Proper email goes here, with HTML templates and all"#,
							token = verification_token,
							email = to_email
						)),
				)
				.singlepart(
					SinglePart::base64()
						.header(header::ContentType(
							"text/plain; charset=utf8".parse().unwrap(),
						))
						.body(format!(
							r#"Verify your account 4 pizza.
							Here's a simple link to use for the token: https://accounts.vicara.co/auth/verify?token={token}&email={email}
							This token expires in 24 hours.
							TODO: Proper email goes here, with HTML templates and all"#,
							token = verification_token,
							email = to_email
						)),
				),
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

pub fn send_sign_up_completed_mail(config: Settings, to_email: String) {
	let from = format!("Bytesonus <{}>", config.email.from)
		.parse()
		.unwrap();
	let to = to_email.parse().unwrap();

	let email = Message::builder()
		.from(from)
		.to(to)
		.subject("Verify your email")
		.multipart(
			MultiPart::alternative()
				.singlepart(
					SinglePart::base64()
						.header(header::ContentType(
							"text/html; charset=utf8".parse().unwrap(),
						))
						.body("Cool. Account verified thenks"),
				)
				.singlepart(
					SinglePart::base64()
						.header(header::ContentType(
							"text/plain; charset=utf8".parse().unwrap(),
						))
						.body("Cool. Account verified thenks"),
				),
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
			log::info!(target: "emails", "Sign up confirmation email to {} sent successfully!", to_email);
		}
		Err(err) => {
			log::error!(target: "emails", "Could not send email to {}: {:#?}", to_email, err);
		}
	}
}
