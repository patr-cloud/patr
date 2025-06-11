use models::api::auth::*;

use crate::prelude::*;

// pub async fn login(
// 	username: String,
// 	password: String,
// 	mfa_otp: Option<String>,
// ) -> Result<(), ServerFnError> {

// }

/// The Login Page
#[component]
pub fn LoginPage(to: String) -> Element {
	rsx! {
		PageContainer { class: "bg-onboard", LoginForm {} }
	}
}

/// The login form component. This is the form that the user uses to log in to
/// the application.
#[component]
pub fn LoginForm() -> Element {
	let mut username = use_signal(|| "".to_owned());
	let mut password = use_signal(|| "".to_owned());

	let mut username_error = use_signal(|| "".to_owned());
	let mut password_error = use_signal(|| "".to_owned());

	let mut loading = use_signal(|| false);

	let on_submit_login = move |ev: Event<FormData>| {
		loading.set(true);
		username_error.set("".to_owned());
		password_error.set("".to_owned());

		let LoginRequest {
			user_id,
			password,
			mfa_otp,
		} = serde_json::from_value(
			// serde_json::to_value(ev.values())
			// 	.expect("failed to parse login request. Most likely the form values are not
			// valid"),
			Default::default(),
		)
		.expect("failed to parse login request");

		if user_id.is_empty() {
			error!("no email");
			username_error.set("Username cannot be empty".to_owned());
			loading.set(false);
		}

		if password.is_empty() {
			error!("no password");
			password_error.set("Password cannot be empty".to_owned());
			loading.set(false);
		}

		spawn(async move {
			_ = make_request(
				ApiRequest::<LoginRequest>::builder()
					.path(LoginPath)
					.query(())
					.headers(LoginRequestHeaders {
						user_agent: UserAgent::from_static("TODO"),
					})
					.body(LoginRequest {
						user_id,
						password,
						mfa_otp,
					})
					.build(),
			)
			.await
			.map(|response| {
				use_context::<Signal<AuthState>>().set(AuthState::LoggedIn {
					access_token: response.body.access_token,
					refresh_token: response.body.refresh_token,
					last_used_workspace_id: None,
				});
			});
		});

		loading.set(false);
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
					name: "user_id",
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

				input { name: "mfa_otp", r#type: "hidden" }

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
