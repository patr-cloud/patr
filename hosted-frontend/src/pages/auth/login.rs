use ev::SubmitEvent;
use models::api::auth::*;

use crate::prelude::*;

/// The login form component. This is the form that the user uses to log in to
/// the application.
#[component]
pub fn LoginForm() -> impl IntoView {
	let (_, set_auth_state) = AuthState::load();
	let app_type = expect_context::<AppType>();

	let username = create_rw_signal("".to_owned());
	let password = create_rw_signal("".to_owned());

	let username_error = create_rw_signal("".to_owned());
	let password_error = create_rw_signal("".to_owned());

	let loading = create_rw_signal(false);

	let handle_errors = move |error| match error {
		ServerFnError::WrappedServerError(ErrorType::UserNotFound) => {
			username_error.set("User Not Found".to_owned());
			password_error.set("".to_owned());
		}
		ServerFnError::WrappedServerError(ErrorType::InvalidPassword) => {
			username_error.set("".to_owned());
			password_error.set("Wrong Password".to_owned());
		}
		ServerFnError::Deserialization(msg) => {
			username_error.set("".to_owned());
			password_error.set(msg);
		}
		e => {
			username_error.set("".to_owned());
			password_error.set(e.to_string());
		}
	};

	let on_submit_login = move |ev: SubmitEvent| {
		ev.prevent_default();
		loading.set(true);
		username_error.set("".to_string());
		password_error.set("".to_string());

		if username.get().is_empty() {
			username_error.set("Please Provide a User Name".to_owned());
			return;
		}

		if password.get().is_empty() {
			password_error.set("Please Provide a Password".to_owned());
			return;
		}

		spawn_local(async move {
			let response = login(username.get_untracked(), password.get_untracked(), None).await;

			match response {
				Ok(LoginResponse {
					access_token,
					refresh_token,
				}) => {
					set_auth_state.set(Some(AuthState::LoggedIn {
						access_token,
						refresh_token,
						last_used_workspace_id: Some(Uuid::nil()),
					}));
					use_navigate()(
						&AppRoutes::LoggedInRoute(LoggedInRoute::Home).to_string(),
						NavigateOptions::default(),
					);
				}
				Err(err) => {
					logging::log!("{:#?}", err);
					handle_errors(err);
				}
			}
			loading.set(false);
		})
	};

	view! {
		<form on:submit={on_submit_login} class="box-onboard text-white">
			<div class="flex justify-between items-baseline mb-lg w-full">
				<h1 class="text-primary text-xl text-medium">"Sign In"</h1>
				<div class="text-white text-thin flex items-start justify-start">
					<p>"New User? "</p>
					<Link to={"/sign-up".to_owned()} r#type={Variant::Link}>
						"Sign Up"
					</Link>
				</div>
			</div>

			<div class="flex flex-col items-start justify-start w-full gap-md">
				<Input
					name="user_id"
					class="w-full"
					id="user_id"
					r#type={InputType::Text}
					placeholder="Username/Email"
					disabled={Signal::derive(move || loading.get())}
					start_icon={Some(
						IconProps::builder().icon(IconType::User).size(Size::ExtraSmall).build(),
					)}
					on_input={Box::new(move |ev| {
						username.set(event_target_value(&ev));
					})}
					value={username}
				/>

				<Show when={move || !username_error.get().is_empty()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						{move || username_error.get()}
					</Alert>
				</Show>

				<Input
					name="password"
					class="w-full"
					id="password"
					r#type={InputType::Password}
					placeholder="Password"
					start_icon={Some(
						IconProps::builder().icon(IconType::Shield).size(Size::ExtraSmall).build(),
					)}
					disabled={Signal::derive(move || loading.get())}
					on_input={Box::new(move |ev| {
						password.set(event_target_value(&ev));
					})}
					value={password}
				/>

				<input name="mfa_otp" type="hidden"/>
				<Show when={move || !password_error.get().is_empty()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						{move || password_error.get()}
					</Alert>
				</Show>
			</div>

			{
				match app_type {
					AppType::SelfHosted => view! {}.into_view(),
					AppType::Managed => view! {
						<div class="flex justify-between items-center w-full pt-xs">
							<Link to={"/forgot-password".to_owned()} r#type={Variant::Link}>
								"Forgot Password?"
							</Link>
						</div>
					}
					.into_view()
				}
			}

			<Show
				when=move || !loading.get()
				fallback=move || view! {
					<Link
						r#type={Variant::Button}
						class="ml-auto"
						style_variant={LinkStyleVariant::Contained}
						disabled={true}
					>
						"LOADING"
					</Link>
				}
			>
				<Link
					should_submit=true
					r#type={Variant::Button}
					class="btn ml-auto mt-md"
					style_variant={LinkStyleVariant::Contained}
				>
					"LOGIN"
				</Link>
			</Show>
		</form>
	}
}
