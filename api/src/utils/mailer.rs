use crate::utils::settings::Settings;

use lettre::{
	header,
	message::{MultiPart, SinglePart},
	transport::smtp::authentication::Credentials,
	Message,
	SmtpTransport,
	Transport,
};

/*
api_macros::email_template!("test");

pub fn send_test_email(config: Settings) -> bool {
	let from = "Test <rakshith@vicara.co>".parse().unwrap();
	let to = "rakshith.ravi@vicara.co".parse().unwrap();

	let message = Message::builder()
		.from(from)
		.to(to)
		.subject("test")
		.multipart(TestEmail::render(String::from("no")))
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
	match mailer.send(&message) {
		Ok(data) => {
			println!("{:#?}", data);
		}
		Err(err) => {
			eprintln!("{:#?}", err);
		}
	}
	true
}
*/

#[allow(clippy::useless_format)]
pub fn send_email_verification_mail(
	config: Settings,
	email: String,
	verification_token: String,
) {
	let from = format!("Bytesonus <{}>", config.email.from)
		.parse()
		.unwrap();
	let to = email.parse().unwrap();

	let message = Message::builder()
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
							email = email
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
							email = email
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
	/*
	match mailer.send(&email) {
		Ok(_) => {
			log::info!(target: "emails", "Verification email to {} sent successfully!", to_email);
		}
		Err(err) => {
			log::error!(target: "emails", "Could not send email to {}: {:#?}", to_email, err);
		}
	}
	*/
}

#[allow(clippy::useless_format)]
pub fn send_backup_registration_mail(config: Settings, email: String) {
	let from = format!("Bytesonus <{}>", config.email.from)
		.parse()
		.unwrap();
	let to = email.parse().unwrap();

	let message = Message::builder()
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
							r#"Dis account is now a backup email for account, kewl?"#,
						)),
				)
				.singlepart(
					SinglePart::base64()
						.header(header::ContentType(
							"text/plain; charset=utf8".parse().unwrap(),
						))
						.body(format!(
							r#"Dis account is now a backup email for account, kewl?"#,
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
	/*
	match mailer.send(&email) {
		Ok(_) => {
			log::info!(target: "emails", "Verification email to {} sent successfully!", email);
		}
		Err(err) => {
			log::error!(target: "emails", "Could not send email to {}: {:#?}", email, err);
		}
	}
	*/
}

#[allow(clippy::useless_format)]
pub fn send_sign_up_completed_mail(config: Settings, email: String) {
	let from = format!("Bytesonus <{}>", config.email.from)
		.parse()
		.unwrap();
	let to = email.parse().unwrap();

	let message = Message::builder()
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
	/*
	match mailer.send(&email) {
		Ok(_) => {
			log::info!(target: "emails", "Sign up confirmation email to {} sent successfully!", email);
		}
		Err(err) => {
			log::error!(target: "emails", "Could not send email to {}: {:#?}", email, err);
		}
	}
	*/
}

#[allow(clippy::useless_format)]
pub fn send_password_reset_requested_mail(
	config: Settings,
	email: String,
	verification_token: String,
) {
	let from = format!("Bytesonus <{}>", config.email.from)
		.parse()
		.unwrap();
	let to = email.parse().unwrap();

	let message = Message::builder()
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
						.body(format!("You asked for a password reset? Here's your token: {}", verification_token)),
				)
				.singlepart(
					SinglePart::base64()
						.header(header::ContentType(
							"text/plain; charset=utf8".parse().unwrap(),
						))
						.body(format!("You asked for a password reset? Here's your token: {}", verification_token)),
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
	/*
	match mailer.send(&email) {
		Ok(_) => {
			log::info!(target: "emails", "Sign up confirmation email to {} sent successfully!", email);
		}
		Err(err) => {
			log::error!(target: "emails", "Could not send email to {}: {:#?}", email, err);
		}
	}
	*/
}

#[allow(clippy::useless_format)]
pub fn send_password_changed_notification_mail(
	config: Settings,
	email: String,
) {
	let from = format!("Bytesonus <{}>", config.email.from)
		.parse()
		.unwrap();
	let to = email.parse().unwrap();

	let message = Message::builder()
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
						.body("Your password has been reset. Deal with it"),
				)
				.singlepart(
					SinglePart::base64()
						.header(header::ContentType(
							"text/plain; charset=utf8".parse().unwrap(),
						))
						.body("Your password has been reset. Deal with it"),
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
	match mailer.send(&message) {
		Ok(_) => {
			log::info!(target: "emails", "Sign up confirmation email to {} sent successfully!", email);
		}
		Err(err) => {
			log::error!(target: "emails", "Could not send email to {}: {:#?}", email, err);
		}
	}
}

#[allow(clippy::useless_format)]
pub fn send_domain_verified_mail(
	config: Settings,
	email: String,
	domain: String,
) {
	let from = format!("Bytesonus <{}>", config.email.from)
		.parse()
		.unwrap();
	let to = email.parse().unwrap();

	let message = Message::builder()
		.from(from)
		.to(to)
		.subject("Domain verified")
		.multipart(
			MultiPart::alternative()
				.singlepart(
					SinglePart::base64()
						.header(header::ContentType(
							"text/html; charset=utf8".parse().unwrap(),
						))
						.body(format!("Domain `{}` verified", domain)),
				)
				.singlepart(
					SinglePart::base64()
						.header(header::ContentType(
							"text/plain; charset=utf8".parse().unwrap(),
						))
						.body(format!("Domain `{}` verified", domain)),
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

	/*
	// Send the email
	match mailer.send(&message) {
		Ok(_) => {
			log::info!(target: "emails", "Sign up confirmation email to {} sent successfully!", email);
		}
		Err(err) => {
			log::error!(target: "emails", "Could not send email to {}: {:#?}", email, err);
		}
	}
	*/
}

#[allow(clippy::useless_format)]
pub fn send_domain_unverified_mail(
	config: Settings,
	email: String,
	domain: String,
) {
	let from = format!("Bytesonus <{}>", config.email.from)
		.parse()
		.unwrap();
	let to = email.parse().unwrap();

	let message = Message::builder()
		.from(from)
		.to(to)
		.subject("Domain unverified")
		.multipart(
			MultiPart::alternative()
				.singlepart(
					SinglePart::base64()
						.header(header::ContentType(
							"text/html; charset=utf8".parse().unwrap(),
						))
						.body(format!("Domain `{}` unverified", domain)),
				)
				.singlepart(
					SinglePart::base64()
						.header(header::ContentType(
							"text/plain; charset=utf8".parse().unwrap(),
						))
						.body(format!("Domain `{}` unverified", domain)),
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

	/*
	// Send the email
	match mailer.send(&message) {
		Ok(_) => {
			log::info!(target: "emails", "Sign up confirmation email to {} sent successfully!", email);
		}
		Err(err) => {
			log::error!(target: "emails", "Could not send email to {}: {:#?}", email, err);
		}
	}
	*/
}
