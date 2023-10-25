use std::rc::Rc;

use leptos_router::use_query_map;
use leptos_use::use_debounce_fn_with_arg;
use models::{
	api::auth::{
		CreateAccountRequest,
		CreateAccountResponse,
		IsEmailValidQuery,
		IsEmailValidRequest,
		IsEmailValidResponse,
		IsUsernameValidQuery,
		IsUsernameValidRequest,
		IsUsernameValidResponse,
		RecoveryMethod,
	},
	ApiErrorResponse,
	ApiErrorResponseBody,
	ApiRequest,
	ApiSuccessResponse,
	ErrorType,
};

use crate::prelude::*;

/// The login page
#[component]
pub fn SignUp() -> impl IntoView {
	let first_name_ref = create_node_ref();
	let last_name_ref = create_node_ref();
	let username_ref = create_node_ref();
	let query = use_query_map();

	let email_ref = create_node_ref();
	let password_ref = create_node_ref();
	let confirm_password_ref = create_node_ref();

	let show_password = create_rw_signal(false);
	let first_name_error = create_rw_signal(String::from(""));
	let last_name_error = create_rw_signal(String::from(""));
	let username_default = query
		.get_untracked()
		.get("username")
		.cloned()
		.unwrap_or_default();
	let username_error_type = create_rw_signal(NotificationType::Error);
	let username_error = create_rw_signal(String::from(""));
	let username_verifying = create_rw_signal(false);
	let email_default = query
		.get_untracked()
		.get("email")
		.cloned()
		.unwrap_or_default();
	let email_error = create_rw_signal(String::from(""));
	let email_error_type = create_rw_signal(NotificationType::Error);
	let email_verifying = create_rw_signal(false);
	let password_error = create_rw_signal(String::from(""));
	let confirm_password_error = create_rw_signal(String::from(""));
	let show_login_username_button = create_rw_signal(false);

	let handle_errors = move |error, message| match error {
		ErrorType::InvalidPassword => {
			password_error.set(message);
		}
		ErrorType::UserNotFound => {
			username_error.set(error.message().into());
			show_login_username_button.set(true);
		}
		ErrorType::InvalidEmail => {
			email_error.set(message);
		}
		_ => {
			confirm_password_error.set(message);
		}
	};

	let username_valid_action = create_action(move |username: &String| {
		let username = username.clone();
		async move {
			// let result = make_request(
			// 	ApiRequest::<IsUsernameValidRequest>::builder()
			// 		.path(Default::default())
			// 		.query(IsUsernameValidQuery { username })
			// 		.headers(())
			// 		.body(IsUsernameValidRequest)
			// 		.build(),
			// )
			// .await;

			// let IsUsernameValidResponse { available } = match result {
			// 	Ok(ApiSuccessResponse {
			// 		status_code: _,
			// 		headers: (),
			// 		body,
			// 	}) => body,
			// 	Err(ApiErrorResponse {
			// 		status_code: _,
			// 		body:
			// 			ApiErrorResponseBody {
			// 				success: _,
			// 				error,
			// 				message,
			// 			},
			// 	}) => {
			// 		handle_errors(error, message);
			// 		return;
			// 	}
			// };

			// if !available {
			// 	username_error.set("Username is already taken".to_string());
			// 	username_error_type.set(NotificationType::Error);
			// } else {
			// 	username_error.set("Username is available".to_string());
			// 	username_error_type.set(NotificationType::Success);
			// }
			// if username_verifying.get_untracked() {
			// 	username_verifying.set(false);
			// }
		}
	});

	let email_valid_action = create_action(move |email: &String| {
		let email = email.clone();
		async move {
			// let result = make_request(
			// 	ApiRequest::<IsEmailValidRequest>::builder()
			// 		.path(Default::default())
			// 		.query(IsEmailValidQuery { email })
			// 		.headers(())
			// 		.body(IsEmailValidRequest)
			// 		.build(),
			// )
			// .await;

			// let IsEmailValidResponse { available } = match result {
			// 	Ok(ApiSuccessResponse {
			// 		status_code: _,
			// 		headers: (),
			// 		body,
			// 	}) => body,
			// 	Err(ApiErrorResponse {
			// 		status_code: _,
			// 		body:
			// 			ApiErrorResponseBody {
			// 				success: _,
			// 				error,
			// 				message,
			// 			},
			// 	}) => {
			// 		handle_errors(error, message);
			// 		return;
			// 	}
			// };

			// if !available {
			// 	email_error.set("Email is already taken".to_string());
			// 	email_error_type.set(NotificationType::Error);
			// } else {
			// 	email_error.set("Email is available".to_string());
			// 	email_error_type.set(NotificationType::Success);
			// }
			// if email_verifying.get_untracked() {
			// 	email_verifying.set(false);
			// }
		}
	});

	let sign_up_action = create_action(
		move |(first_name, last_name, username, email, password): &(
			String,
			String,
			String,
			String,
			String,
		)| {
			let first_name = first_name.clone();
			let last_name = last_name.clone();
			let username = username.clone();
			let recovery_email = email.clone();
			let password = password.clone();
			async move {
				// let result = make_request(
				// 	ApiRequest::<CreateAccountRequest>::builder()
				// 		.path(Default::default())
				// 		.query(())
				// 		.headers(())
				// 		.body(CreateAccountRequest {
				// 			first_name,
				// 			last_name,
				// 			username,
				// 			password,
				// 			recovery_method: RecoveryMethod::Email { recovery_email },
				// 		})
				// 		.build(),
				// )
				// .await;

				// let CreateAccountResponse = match result {
				// 	Ok(ApiSuccessResponse {
				// 		status_code: _,
				// 		headers: (),
				// 		body,
				// 	}) => body,
				// 	Err(ApiErrorResponse {
				// 		status_code: _,
				// 		body:
				// 			ApiErrorResponseBody {
				// 				success: _,
				// 				error,
				// 				message,
				// 			},
				// 	}) => {
				// 		handle_errors(error, message);
				// 		return;
				// 	}
				// };
			}
		},
	);

	let check_username_valid = use_debounce_fn_with_arg(
		move |username: String| {
			if !username.is_empty() {
				username_valid_action.dispatch(username);
			} else {
				username_error.set("".into());
			}
		},
		MaybeSignal::Static(500f64),
	);

	let check_email_valid = use_debounce_fn_with_arg(
		move |email: String| {
			if !email.is_empty() {
				email_valid_action.dispatch(email);
			} else {
				email_error.set("".into());
			}
		},
		MaybeSignal::Static(500f64),
	);

	let check_confirm_password_valid = use_debounce_fn_with_arg(
		move |confirm_password: String| {
			if confirm_password.is_empty() {
				confirm_password_error.set("Please confirm your Password again".into());
				return;
			}

			if password_ref
				.get()
				.map(|element: HtmlElement<html::Input>| element.value())
				.unwrap() != confirm_password
			{
				confirm_password_error.set("Passwords do not match".into());
			}
		},
		MaybeSignal::Static(500f64),
	);

	let handle_login_username = |e: &ev::MouseEvent| {
		e.prevent_default();
	};

	let handle_sign_up = move |e: ev::SubmitEvent| {
		e.prevent_default();

		let first_name = first_name_ref
			.get()
			.map(|value: HtmlElement<html::Input>| value.value())
			.unwrap();
		let last_name = last_name_ref
			.get()
			.map(|value: HtmlElement<html::Input>| value.value())
			.unwrap();
		let username = username_ref
			.get()
			.map(|value: HtmlElement<html::Input>| value.value())
			.unwrap();
		let email = email_ref
			.get()
			.map(|value: HtmlElement<html::Input>| value.value())
			.unwrap();
		let password = password_ref
			.get()
			.map(|value: HtmlElement<html::Input>| value.value())
			.unwrap();
		let confirm_password = confirm_password_ref
			.get()
			.map(|value: HtmlElement<html::Input>| value.value())
			.unwrap();

		let mut invalid_data = false;

		if first_name.is_empty() {
			first_name_error.set("First Name cannot be empty".into());
			_ = first_name_ref.get().unwrap().focus();
			invalid_data = true;
		}

		if last_name.is_empty() {
			last_name_error.set("Last Name cannot be empty".into());
			_ = last_name_ref.get().unwrap().focus();
			invalid_data = true;
		}

		if username.is_empty() {
			username_error.set("Username cannot be empty".into());
			_ = username_ref.get().unwrap().focus();
			invalid_data = true;
		}

		if email.is_empty() {
			email_error.set("Email cannot be empty".into());
			_ = email_ref.get().unwrap().focus();
			invalid_data = true;
		}

		if password.is_empty() {
			password_error.set("Password cannot be empty".into());
			_ = password_ref.get().unwrap().focus();
			invalid_data = true;
		}

		if confirm_password.is_empty() {
			confirm_password_error.set("Please confirm your Password again".into());
			_ = confirm_password_ref.get().unwrap().focus();
			invalid_data = true;
		}

		if password != confirm_password {
			confirm_password_error.set("Passwords do not match".into());
			_ = confirm_password_ref.get().unwrap().focus();
			invalid_data = true;
		}

		if invalid_data {
			return;
		}

		sign_up_action.dispatch((first_name, last_name, username, email, password));
	};

	let sign_up_loading = sign_up_action.pending();

	view! {
		<form class="box-onboard txt-white fc-fs-fs" on:submit=handle_sign_up>
			<div class="fr-sb-bl mb-lg full-width">
				<h1 class="txt-primary txt-xl txt-medium">{"Sign In"}</h1>
				<p class="txt-white txt-thin fr-fs-fs">
					Already have an account?
					<Link
						disabled={sign_up_loading}
						to=AppRoute::LoggedOutRoute(LoggedOutRoute::Login)
						class="ml-xs"
					>
						Login
					</Link>
				</p>
			</div>
			<div class="fr-ct-fs full-width">
				<div class="fc-fs-fs grid-col-6 pr-xxs">
					<Input
						r#ref=first_name_ref
						class="py-xs"
						r#type="text"
						id="firstName"
						disabled={sign_up_loading}
						placeholder="First Name"
						on_input=Box::new(move |_| {
							first_name_error.update(|value| value.clear());
						})
						start_icon={
							Some(IconProps::builder()
								.icon(IconType::User)
								.size(Size::ExtraSmall)
								.build())
						}
					/>
					{move || {
						first_name_error
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
				</div>
				<div class="fc-fs-fs grid-col-6 pl-xxs">
					<Input
						r#ref=last_name_ref
						class="py-xs"
						r#type="text"
						id="lastName"
						disabled={sign_up_loading}
						placeholder="Last Name"
						on_input=Box::new(move |_| {
							last_name_error.update(|value| value.clear());
						})
						start_icon={
							Some(IconProps::builder()
								.icon(IconType::User)
								.size(Size::ExtraSmall)
								.build())
						}
					/>
					{move || {
						last_name_error
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
				</div>
			</div>
			<Input
				r#ref=username_ref
				r#type="text"
				class="mt-lg full-width"
				disabled={sign_up_loading}
				id="username"
				value=username_default
				loading=username_verifying
				on_input=Box::new(move |ev| {
					let value = event_target_value(&ev);
					if username_verifying.get_untracked() != !value.is_empty() {
						username_verifying.set(!value.is_empty());
					}
					check_username_valid(value);
					username_error.update(|password| password.clear());
				})
				placeholder="Username"
				start_icon={
					Some(IconProps::builder()
						.icon(IconType::User)
						.size(Size::ExtraSmall)
						.build())
				}
			/>
			<div class="fr-fs-ct">
				{move || {
					let username_error = username_error.get();
					let username_error_type = username_error_type.get();
					username_error
						.some_if_not_empty()
						.map(|username| {
							view! {
								<Alert
									r#type=username_error_type
									class="mt-xs"
									message=username
									/>
							}
						})
				}}
				{move || show_login_username_button
					.with(|value| {
						value.then(move || view! {
							<Link
								disabled={sign_up_loading}
								on_click=Box::new(handle_login_username)
								class="ml-sm txt-underline txt-medium mt-xs"
							>
								Login as {move || {
									username_ref
										.get()
										.unwrap()
										.value()
								}}?
							</Link>
						})
					})
				}
			</div>
			<Input
				r#ref=email_ref
				id="email"
				class="mt-lg full-width"
				r#type="email"
				disabled=sign_up_loading
				loading=email_verifying
				value=email_default
				on_input=Box::new(move |ev| {
					let value = event_target_value(&ev);
					// If the value is empty, we don't want to show the loading
					// icon. So we set the value of the loading icon to the
					// input having a value. If the input has a value, then
					// loading is true, else it is false.
					if email_verifying.get_untracked() != !value.is_empty() {
						email_verifying.set(!value.is_empty());
					}
					check_email_valid(value);
					email_error.update(|password| password.clear());
				})
				placeholder="patron@email.com"
				start_icon={
					Some(IconProps::builder()
						.icon(IconType::Mail)
						.size(Size::ExtraSmall)
						.build())
				}
				/>
			{move || {
				email_error
					.get()
					.some_if_not_empty()
					.map(|email| {
						view! {
							<Alert
								r#type=NotificationType::Error
								class="mt-xs"
								message=email
								/>
						}
					})
			}}
			<Input
				r#ref=password_ref
				class="mt-md full-width"
				r#type={MaybeSignal::derive(move || if show_password.get() {
					"text".to_owned()
				} else {
					"password".to_owned()
				})}
				on_input=Box::new(move |ev| {
					let value = event_target_value(&ev);
					if !value.is_empty() {
						check_confirm_password_valid(value);
					}
					password_error.update(|password| password.clear());
				})
				id="password"
				placeholder="Password"
				disabled={sign_up_loading}
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
			<Input
				r#ref=confirm_password_ref
				class="mt-md full-width"
				r#type={MaybeSignal::derive(move || if show_password.get() {
					"text".to_owned()
				} else {
					"password".to_owned()
				})}
				on_input=Box::new(move |_| {
					confirm_password_error.update(|password| password.clear());
				})
				id="password"
				placeholder="Confirm Password"
				disabled={sign_up_loading}
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
				confirm_password_error
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
			<div class="fr-fe-ct full-width pt-md">
				<Link
					disabled={sign_up_loading}
					// on_click={move || {
					// 	isLandingPage
					// 		? confirmOtp && confirmOtp()
					// 		: navigate(PublicPath.CONFIRM_SIGN_UP)
					// }}
					class="btn mr-xs txt-thin txt-xs"
					>
					ALREADY HAVE AN OTP?
				</Link>
				{move || {
					if sign_up_loading.get() {
						view! {
							<Spinner class="mx-xl" />
						}
					} else {
						view! {
							<Link
								disabled={sign_up_loading}
								r#type="submit"
								variant=LinkVariant::Contained>
								NEXT
							</Link>
						}
					}
				}}
			</div>
		</form>
	}
}
