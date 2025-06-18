use models::api::auth::*;
use serde_json::Value;

use crate::prelude::*;

#[server]
pub async fn login(
	user_id: String,
	password: String,
	mfa_otp: Option<String>,
) -> Result<LoginResponse, ServerFnError<ErrorType>> {
	use axum::http::request::Parts;
	use models::utils::Headers;

	let Ok(parts) = extract::<Parts, _>().await;

	// Add cookie to response
	let response = make_api_call(
		ApiRequest::<LoginRequest>::builder()
			.path(LoginPath)
			.query(())
			.headers(
				LoginRequestHeaders::from_header_map(&parts.headers)
					.map_err(|err| ServerFnError::Request(err.to_string()))?,
			)
			.body(LoginRequest {
				user_id,
				password,
				mfa_otp,
			})
			.build(),
	)
	.await?;

	Ok(LoginResponse {
		access_token: response.body.access_token,
		refresh_token: response.body.refresh_token,
	})
}

/// The Login Page
#[component]
pub fn LoginPage(
	to: String,
	username_error: String,
	password_error: String,
	show_mfa_input: bool,
) -> Element {
	rsx! {
		PageContainer { class: "bg-onboard",
			LoginForm { username_error, password_error, show_mfa_input }
		}
	}
}

/// The login form component. This is the form that the user uses to log in to
/// the application.
#[component]
pub fn LoginForm(username_error: String, password_error: String, show_mfa_input: bool) -> Element {
	let mut username = use_signal(|| "".to_owned());
	let mut password = use_signal(|| "".to_owned());

	let mut username_error = use_signal(|| username_error);
	let mut password_error = use_signal(|| password_error);

	let mut loading = use_signal(|| false);

	let on_submit_login = move |ev: Event<FormData>| {
		loading.set(true);
		username_error.set("".to_owned());
		password_error.set("".to_owned());

		let Ok(request) = serde_json::from_value::<LoginRequest>(
			ev.values()
				.into_iter()
				.filter_map(|(key, value)| Some((key, value.0.into_iter().next()?)))
				.collect::<Value>(),
		)
		.inspect_err(|err| {
			error!("failed to parse login request: {}", err);
		}) else {
			loading.set(false);
			return;
		};
		info!("login request: {:?}", request);

		if request.user_id.is_empty() {
			error!("no email");
			username_error.set("Username cannot be empty".to_owned());
			loading.set(false);
		}

		if request.password.is_empty() {
			error!("no password");
			password_error.set("Password cannot be empty".to_owned());
			loading.set(false);
		}

		spawn(async move {
			_ = login(request.user_id, request.password, request.mfa_otp)
				.await
				.map(|response| {
					use_context::<Signal<AuthState>>().set(AuthState::LoggedIn {
						access_token: response.access_token,
						refresh_token: response.refresh_token,
						last_used_workspace_id: None,
					});
				});

			loading.set(false);
		});
	};

	rsx! {
		form { onsubmit: on_submit_login, class: "box-onboard text-white",
			div { class: "flex justify-between items-baseline mb-lg w-full",
				h1 { class: "text-primary text-xl text-medium", "Sign In" }
				div { class: "text-white text-thin flex items-start justify-start text-sm",
					p { class: "mr-xs", "New User?" }
					button { class: "text-primary text-thin", "Sign Up" }
								// AppLink { to: "/sign-up", "Sign Up" }
				}
			}

			div { class: "flex flex-col items-start justify-start w-full gap-md",
				Input {
					id: "user_id",
					name: "userId",
					value: username.read().clone(),
					oninput: move |ev: Event<FormData>| {
						username.set(ev.value());
					},
					class: "w-full",
					r#type: InputType::Text,
					placeholder: "Username / Email",
					disabled: false,
					start_icon: rsx! {
						Icon { icon: IconType::User, size: Size::ExtraSmall }
					},
				}

				if let Some(value) = username_error.read().clone().some_if_not_empty() {
					Alert { class: "mt-xs", r#type: AlertType::Error, {value} }
				}

				PasswordInput {
					id: "password",
					name: "password",
					value: password.read().clone(),
					oninput: move |ev: Event<FormData>| {
						password.set(ev.value());
					},
					class: "w-full",
					placeholder: "Password",
					disabled: false,
					start_icon: rsx! {
						Icon { icon: IconType::Shield, size: Size::ExtraSmall }
					},
				}

				input { name: "mfaOtp", r#type: "hidden" }

				if let Some(value) = password_error.read().clone().some_if_not_empty() {
					Alert { class: "mt-xs", r#type: AlertType::Error, {value} }
				}
			}

			if *loading.read() {
				Spinner { class: "ml-auto" }
			} else {
				Button {
					r#type: ButtonType::Submit,
					class: "btn ml-auto mt-md",
					variant: LinkStyleVariant::Contained,
					"LOGIN"
				}
			}
		}
	}
}
