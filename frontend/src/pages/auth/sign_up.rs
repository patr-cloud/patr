use ev::SubmitEvent;
use leptos_use::{signal_debounced_with_options, utils::DebounceOptions};
use models::api::auth::*;

use crate::prelude::*;

/// Server Function to sign up a new user
#[server(CreateAccount, endpoint = "auth/sign-up")]
pub async fn sign_up(
	first_name: String,
	last_name: String,
	email: String,
	username: String,
	password: String,
) -> Result<(), ServerFnError<ErrorType>> {
	make_api_call::<CreateAccountRequest>(
		ApiRequest::builder()
			.path(CreateAccountPath)
			.query(())
			.headers(CreateAccountRequestHeaders {
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(CreateAccountRequest {
				username,
				password,
				first_name,
				last_name,
				recovery_method: RecoveryMethod::Email {
					recovery_email: email,
				},
			})
			.build(),
	)
	.await?;

	Ok(())
}

#[component]
pub fn SignUpForm(
	/// The query params for the page
	query: SignUpQuery,
) -> impl IntoView {
	let SignUpQuery {
		next,
		first_name,
		last_name,
		username,
		email,
	} = query;

	let app_type = expect_context::<AppType>();

	let first_name = create_rw_signal(first_name.unwrap_or_else(|| "".to_owned()));
	let name_error = Signal::derive(move || {
		first_name
			.get()
			.is_empty()
			.then_some("Name cannot be empty".to_owned())
			.unwrap_or_default()
	});

	let last_name = create_rw_signal(last_name.unwrap_or_else(|| "".to_owned()));

	let email = create_rw_signal(email.unwrap_or_else(|| "".to_owned()));
	let email_error = create_rw_signal("".to_owned());
	// let email_checking = create_resource(
	// 	move || {
	// 		signal_debounced_with_options(
	// 			email,
	// 			constants::DEFAULT_DEBOUNCE_TIME,
	// 			DebounceOptions::default().max_wait(Some(constants::MAX_DEBOUNCE_TIME)),
	// 		)
	// 		.get()
	// 	},
	// 	move |email| async move {
	// 		if email.is_empty() {
	// 			email_error.set("Email cannot be empty".to_owned());
	// 			return;
	// 		}

	// 		let Ok(IsEmailValidResponse { available }) =
	// make_api_call::<IsEmailValidRequest>( 			ApiRequest::builder()
	// 				.path(IsEmailValidPath)
	// 				.query(IsEmailValidQuery { email })
	// 				.headers(IsEmailValidRequestHeaders {
	// 					user_agent: UserAgent::from_static("hyper/0.12.2"),
	// 				})
	// 				.body(IsEmailValidRequest)
	// 				.build(),
	// 		)
	// 		.await
	// 		.map(|response| response.body) else {
	// 			email_error.set("".to_owned());
	// 			return;
	// 		};

	// 		if !available {
	// 			email_error.set("User Not Found".to_owned());
	// 		} else {
	// 			email_error.set("".to_owned());
	// 		}
	// 	},
	// );

	let username = create_rw_signal(username.unwrap_or_else(|| "".to_owned()));
	let username_error = create_rw_signal("".to_owned());
	// let username_checking = create_resource(
	// 	move || {
	// 		signal_debounced_with_options(
	// 			username,
	// 			constants::DEFAULT_DEBOUNCE_TIME,
	// 			DebounceOptions::default().max_wait(Some(constants::MAX_DEBOUNCE_TIME)),
	// 		)
	// 		.get()
	// 	},
	// 	move |username| async move {
	// 		if username.is_empty() {
	// 			username_error.set("Username cannot be empty".to_owned());
	// 			return;
	// 		}

	// 		let Ok(IsUsernameValidResponse { available }) =
	// 			make_api_call::<IsUsernameValidRequest>(
	// 				ApiRequest::builder()
	// 					.path(IsUsernameValidPath)
	// 					.query(IsUsernameValidQuery { username })
	// 					.headers(IsUsernameValidRequestHeaders {
	// 						user_agent: UserAgent::from_static("hyper/0.12.2"),
	// 					})
	// 					.body(IsUsernameValidRequest)
	// 					.build(),
	// 			)
	// 			.await
	// 			.map(|response| response.body)
	// 		else {
	// 			username_error.set("".to_owned());
	// 			return;
	// 		};

	// 		if !available {
	// 			username_error.set("User Not Found".to_owned());
	// 		} else {
	// 			username_error.set("".to_owned());
	// 		}
	// 	},
	// );

	let password = create_rw_signal("".to_owned());
	let password_error = create_rw_signal("".to_owned());
	let password_valid = Signal::derive(move || password.get().len() >= 8);

	let password_confirm = create_rw_signal("".to_owned());
	let password_confirm_error = create_rw_signal("".to_owned());
	let passwords_match = Signal::derive(move || password.get() != password_confirm.get());

	let loading = create_rw_signal(false);

	let on_submit_sign_up = move |ev: SubmitEvent| {
		ev.prevent_default();
		loading.set(true);

		email_error.set("".to_string());
		username_error.set("".to_string());
		password_error.set("".to_string());
		password_confirm_error.set("".to_string());

		if first_name.get().is_empty() || last_name.get().is_empty() {
			loading.set(false);
			return;
		}

		if email.get().is_empty() {
			email_error.set("Email cannot be empty".to_string());
			loading.set(false);
			return;
		}

		if username.get().is_empty() {
			username_error.set("Username cannot be empty".to_string());
			loading.set(false);
			return;
		}

		if password.get().is_empty() {
			password_error.set("Password cannot be empty".to_string());
			loading.set(false);
			return;
		}

		if password_confirm.get().is_empty() {
			password_confirm_error.set("Re-enter your password".to_string());
			loading.set(false);
			return;
		}

		let next = next.clone();

		spawn_local(async move {
			match sign_up(
				first_name.get_untracked(),
				last_name.get_untracked(),
				email.get_untracked(),
				username.get_untracked(),
				password.get_untracked(),
			)
			.await
			{
				Ok(()) => match app_type {
					AppType::SelfHosted => {
						use_navigate()(
							&AppRoutes::LoggedOutRoute(LoggedOutRoute::Login).to_string(),
							Default::default(),
						);
					}
					AppType::Managed => {
						use_navigate()(
							&AppRoutes::LoggedOutRoute(LoggedOutRoute::ConfirmOtp).to_string(),
							Default::default(),
						);
					}
				},
				Err(ServerFnError::WrappedServerError(ErrorType::UsernameUnavailable)) => {
					username_error.set("Username not available".to_owned());
				}
				Err(ServerFnError::WrappedServerError(ErrorType::EmailUnavailable)) => {
					email_error.set("Email not available".to_owned());
				}
				Err(e) => {
					password_error.set(e.to_string());
				}
			}

			loading.set(false);
		})
	};

	view! {
		<div class="box-onboard text-white">
			<div class="flex justify-between items-baseline mb-lg w-full">
				<h1 class="text-primary text-xl text-medium">"Sign Up"</h1>

				<div class="text-white text-thin flex justify-start items-start">
					<p>"Existing User? "</p>
					<Link to="/login" r#type={Variant::Link} class="ml-xs">
						"Login"
					</Link>
				</div>
			</div>

			<form
				on:submit={on_submit_sign_up}
				class="flex flex-col items-start justify-start w-full"
			>
				<div class="flex justify-center items-start w-full">
					<div class="flex flex-col items-start justify-start flex-col-6 pr-xxs">
						<Input
							class="py-xs"
							r#type={InputType::Text}
							id="first_name"
							name="first_name"
							placeholder="First Name"
							value={first_name}
							on_input={Box::new(move |ev| {
								first_name.set(event_target_value(&ev))
							})}
							start_icon={Some(
								IconProps::builder().icon(IconType::User).size(Size::Medium).build(),
							)}
						/>
					</div>

					<div class="flex flex-col items-start justify-start flex-col-6 pl-xxs">
						<Input
							class="py-xs"
							r#type={InputType::Text}
							id="last_name"
							name="last_name"
							placeholder="Last Name"
							value={last_name}
							on_input={Box::new(move |ev| {
								last_name.set(event_target_value(&ev))
							})}
							start_icon={Some(
								IconProps::builder().icon(IconType::User).size(Size::Medium).build(),
							)}
						/>
					</div>
				</div>
				<Show when={move || !name_error.get().is_empty()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						{move || name_error.get()}
					</Alert>
				</Show>

				<Input
					class="w-full mt-lg"
					r#type={InputType::Text}
					id="username"
					name="username"
					placeholder="User Name"
					start_icon={Some(IconProps::builder().icon(IconType::User).build())}
					value={username}
					on_input={Box::new(move |ev| { username.set(event_target_value(&ev)) })}
				/>

				<Show when={move || !username_error.get().is_empty()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						{move || username_error.get()}
					</Alert>
				</Show>

				<Input
					class="w-full mt-lg"
					r#type={InputType::Email}
					name="email"
					id="email"
					placeholder="proton@gmail.com"
					start_icon={Some(IconProps::builder().icon(IconType::Mail).build())}
					value={email}
					on_input={Box::new(move |ev| { email.set(event_target_value(&ev)) })}
				/>

				<Show when={move || !email_error.get().is_empty()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						{move || email_error.get()}
					</Alert>
				</Show>

				<Input
					r#type={InputType::Password}
					id="password"
					name="password"
					placeholder="Password"
					class="w-full mt-xxs"
					start_icon={Some(
						IconProps::builder().icon(IconType::Unlock).size(Size::Small).build(),
					)}
					value={password}
					on_input={Box::new(move |ev| {
						password.set(event_target_value(&ev));
					})}
				/>
				<Show when={move || !password_error.get().is_empty()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						{move || password_error.get()}
					</Alert>
				</Show>

				<Input
					r#type={InputType::Password}
					id="confirmPassword"
					placeholder="Confirm Password"
					class="w-full mt-lg"
					value={password_confirm}
					start_icon={Some(
						IconProps::builder().icon(IconType::Lock).size(Size::Small).build(),
					)}

					on_input={Box::new(move |ev| {
						password_confirm.set(event_target_value(&ev));
					})}
				/>

				<Show when={move || passwords_match.get()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						"Passwords Don't Match"
					</Alert>
				</Show>

				<div class="fr-fe-ct w-full mt-lg">
					{app_type
						.is_managed()
						.then(|| {
							view! {
								<Link class="btn mr-xs" to="/confirm" r#type={Variant::Link}>
									"ALREADY HAVE AN OTP"
								</Link>
							}
								.into_view()
						})}
					<Show
						when={move || !loading.get()}
						fallback={move || {
							view! {
								<Link
									disabled=true
									r#type={Variant::Button}
									style_variant={LinkStyleVariant::Contained}
								>
									"LOADING..."
								</Link>
							}
						}}
					>
						<Link
							disabled={Signal::derive(move || {
								passwords_match.get() || loading.get()
							})}
							r#type={Variant::Button}
							should_submit=true
							style_variant={LinkStyleVariant::Contained}
						>
							"NEXT"
						</Link>
					</Show>
				</div>
			</form>
		</div>
	}
}
