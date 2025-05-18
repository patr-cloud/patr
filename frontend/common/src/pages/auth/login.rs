use crate::prelude::*;

/// The Login Page
#[component]
pub fn LoginPage() -> Element {
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

	let on_submit_login = move |ev: FormEvent| {
		ev.prevent_default();

		loading.set(true);
		username_error.set("".to_owned());
		password_error.set("".to_owned());

		if username.read().is_empty() {
			error!("no email");
			username_error.set("Username cannot be empty".to_owned());
			loading.set(false);
		}

		if password.read().is_empty() {
			error!("no password");
			password_error.set("Password cannot be empty".to_owned());
			loading.set(false);
		}

		// TODO: Submit Form Here
		info!("Submit Form");
	};

	let username_start_icon = rsx! {
		Icon { icon: IconType::User, size: Size::ExtraSmall }
	};

	let password_start_icon = rsx! {
		Icon { icon: IconType::Shield, size: Size::ExtraSmall }
	};

	rsx! {
		form { class: "box-onboard text-white", onsubmit: on_submit_login,
			div { class: "flex justify-between items-baseline mb-lg w-full",
				h1 { class: "text-primary text-xl text-medium", "Sign In" }
				div { class: "text-white text-thin flex items-start justify-start text-sm",
					p { "New User? " }
					a { href: "/sign-up", "Sign Up" }
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
					start_icon: username_start_icon,
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
					start_icon: password_start_icon,
				}

				input { name: "mfa_otp", r#type: "hidden" }

				if let Some(value) = password_error.read().clone().some_if_not_empty() {
					Alert { class: "mt-xs", r#type: AlertType::Error, {value} }
				}
			}

			if *loading.read() {
				Button {
					r#type: ButtonType::Submit,
					class: "btn ml-auto mt-md",
					variant: LinkStyleVariant::Contained,
					"LOGIN"
				}
			} else {
				p { "Loading..." }
			}
		}
	}
}
