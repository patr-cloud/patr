use std::rc::Rc;

use leptos_router::{use_navigate, NavigateOptions};
use models::{
	api::auth::{LoginRequest, LoginResponse},
	ApiErrorResponse,
	ApiErrorResponseBody,
	ApiRequest,
	ApiSuccessResponse,
	ErrorType,
};

use crate::prelude::*;

/// The login page
#[component]
pub fn Login() -> impl IntoView {
	// let set_state = expect_context::<WriteSignal<AppStorage>>();
	let (_state, set_state) = create_signal(AppStorage::LoggedOut);

	let show_password = create_rw_signal(false);
	let show_create_account_button = create_rw_signal(false);

	let show_otp_input = create_rw_signal(false);

	let username_error = create_rw_signal(String::from(""));
	let password_error = create_rw_signal(String::from(""));
	let mfa_otp_error = create_rw_signal(String::from(""));

	let handle_errors = move |error, message| match error {
		ErrorType::MfaRequired => {
			show_otp_input.set(true);
		}
		ErrorType::MfaOtpInvalid => {
			mfa_otp_error.set(message);
		}
		ErrorType::InvalidPassword => {
			password_error.set(message);
		}
		ErrorType::UserNotFound => {
			username_error.set(error.message().into());
			show_create_account_button.set(true);
		}
		_ => {
			password_error.set(message);
		}
	};

	let login_action = create_action(
		move |(username, password, mfa_otp): &(String, String, Option<String>)| {
			let user_id = username.clone();
			let password = password.clone();
			let mfa_otp = mfa_otp.clone();
			async move {
				let result = make_request(
					ApiRequest::<LoginRequest>::builder()
						.path(Default::default())
						.query(())
						.headers(())
						.body(LoginRequest {
							user_id,
							password,
							mfa_otp,
						})
						.build(),
				)
				.await;
				let LoginResponse {
					access_token,
					refresh_token,
				} = match result {
					Ok(ApiSuccessResponse {
						status_code: _,
						headers: (),
						body,
					}) => body,
					Err(ApiErrorResponse {
						status_code: _,
						body:
							ApiErrorResponseBody {
								success: _,
								error,
								message,
							},
					}) => {
						handle_errors(error, message);
						return;
					}
				};

				set_state.set(AppStorage::LoggedIn {
					user_id: Default::default(),
					access_token,
					refresh_token,
					default_workspace: None,
				});
			}
		},
	);

	let login_loading = login_action.pending();

	let user_id_ref = create_node_ref();
	let password_ref = create_node_ref();
	let mfa_otp_ref = create_node_ref();

	let handle_login = move |e: ev::SubmitEvent| {
		e.prevent_default();

		let user_id = user_id_ref
			.get()
			.map(|input: HtmlElement<html::Input>| input.value())
			.unwrap();
		let password = password_ref
			.get()
			.map(|input: HtmlElement<html::Input>| input.value())
			.unwrap();

		let mfa_otp = mfa_otp_ref
			.get()
			.map(|input: HtmlElement<html::Input>| input.value());

		if user_id.is_empty() {
			username_error.set("Username / Email cannot be empty".into());
			_ = user_id_ref.get().unwrap().focus();
			return;
		}

		if password.is_empty() {
			password_error.set("Password cannot be empty".into());
			_ = password_ref.get().unwrap().focus();
			return;
		}

		login_action.dispatch((user_id, password, mfa_otp));
	};

	let handle_create_new_account = move |e: &ev::MouseEvent| {
		e.prevent_default();

		let user_id = user_id_ref
			.get()
			.map(|input: HtmlElement<html::Input>| input.value())
			.unwrap();

		let is_email = user_id.contains('@');

		// navigate to the create new account page with the username
		// pre-filled through setting the state
		let navigate = use_navigate();
		_ = navigate(
			format!(
				"{}?{}",
				AppRoute::LoggedOutRoute(LoggedOutRoute::SignUp)
					.to_string()
					.as_str(),
				serde_urlencoded::to_string([(
					if is_email { "email" } else { "username" },
					user_id.as_str()
				)])
				.unwrap(),
			)
			.as_str(),
			NavigateOptions::default(),
		);
	};

	view! {
		<form class="box-onboard txt-white fc-fs-fs" on:submit=handle_login>
			<div class="fr-sb-bl mb-lg full-width">
				<h1 class="txt-primary txt-xl txt-medium">{"Sign In"}</h1>
				<p class="txt-white txt-thin fr-fs-fs">
					New user?
					<Link
						disabled={login_loading}
						to=AppRoute::LoggedOutRoute(LoggedOutRoute::SignUp)
						class="ml-xs"
					>
						Sign Up
					</Link>
				</p>
			</div>
			<Input
				r#type="text"
				class="full-width"
				disabled={login_loading}
				id="username"
				on_input=Box::new(move |_| {
					username_error.update(|password| password.clear());
				})
				r#ref=user_id_ref
				placeholder="Username/Email"
				start_icon={
					Some(IconProps::builder()
						.icon(IconType::User)
						.size(Size::ExtraSmall)
						.build())
				}
			/>
			<div class="fr-fs-ct">
				{move || {
					username_error
						.get()
						.some_if_not_empty()
						.map(|username| {
							view! {
								<Alert
									r#type=NotificationType::Error
									class="mt-xs"
									message=username
									/>
							}
						})
				}}
				{move || show_create_account_button
					.with(|value| {
						value.then(move || view! {
							<Link
								disabled={login_loading}
								to=AppRoute::LoggedOutRoute(LoggedOutRoute::SignUp)
								on_click=Box::new(handle_create_new_account)
								class="ml-sm txt-underline txt-medium mt-xs"
							>
								Create a new account?
							</Link>
						})
					})
				}
			</div>
			<Input
				class="mt-md full-width"
				r#type={MaybeSignal::derive(move || if show_password.get() {
					"text".to_owned()
				} else {
					"password".to_owned()
				})}
				on_input=Box::new(move |_| {
					password_error.update(|password| password.clear());
				})
				id="password"
				r#ref=password_ref
				placeholder="Password"
				disabled={login_loading}
				start_icon={
					Some(
						IconProps::builder()
							.icon(IconType::Shield)
							.size(Size::ExtraSmall)
							.build()
					)
				}
				end_icon={
					Some(
						IconProps::builder()
							.icon(MaybeSignal::derive(move || {
								if show_password.get() {
									IconType::Eye
								} else {
									IconType::EyeOff
								}
							}))
							.color(Color::Grey)
							.size(Size::ExtraSmall)
							.on_click(Rc::new(move |_| {
								show_password.update(|value| *value = !*value);
							}))
							.build()
					)
				}
			/>
			{move || {
				password_error
					.get()
					.some_if_not_empty()
					.map(|password| {
						view! {
							<Alert
								r#type=NotificationType::Error
								class="mt-xs"
								message={password}
								/>
						}
					})
			}}
			{move || show_otp_input.get().then(|| {
				view! {
					<p class="mt-xl txt-center txt-grey">
						Enter the OTP generated by your authenticator app to log in. <br />
						This additional step is required as "you've" enabled Two-Factor
						Authentication for your account.
					</p>
					<OtpInput
						id="mfa-otp"
						r#ref=mfa_otp_ref
						on_submit=Rc::new(move |_| {
							handle_login(ev::SubmitEvent::new("submit").unwrap());
						})
						disabled={login_loading}
						class="mt-xl"
						/>
				}
			})}
			{move || {
				mfa_otp_error
					.get()
					.some_if_not_empty()
					.map(|mfa_otp| {
						view! {
							<Alert
								r#type=NotificationType::Error
								class="mt-xs"
								message={mfa_otp}
								/>
						}
					})
			}}
			<div class="fr-sb-ct full-width mt-xs">
				<Link
					to=AppRoute::LoggedOutRoute(LoggedOutRoute::ForgotPassword)
					disabled={login_loading}>
					Forgot Password?
				</Link>
			</div>
			{move || if login_loading.get() {
				view! {
					<Spinner class="mt-md mr-xl ml-auto" />
				}
			} else {
				view! {
					<Link
						r#type="submit"
						variant=LinkVariant::Contained
						class="mt-md ml-auto">
						LOGIN
					</Link>
				}
			}}
		</form>
	}
}
