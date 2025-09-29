use codee::string::JsonSerdeCodec;
use leptos::ev::Event;
use leptos_use::{UseCookieOptions, use_cookie_with_options};
use models::frontend::auth::*;

use crate::prelude::*;

/// The server function to log in the user. This will set the access token
/// cookie on the response.
#[server]
async fn login_action(
	user_id: String,
	password: String,
	mfa_otp: Option<String>,
	next: Option<String>,
) -> Result<(), ServerFnError<ErrorType>> {
	use cookie::Cookie;
	use leptos_axum::ResponseOptions;
	use models::api::auth::*;

	let response = make_api_call(
		ApiRequest::<LoginRequest>::builder()
			.path(LoginPath)
			.headers(LoginRequestHeaders {
				user_agent: UserAgent::from_static("TODO"),
			})
			.query(())
			.body(LoginRequest {
				user_id,
				password,
				mfa_otp: None,
			})
			.build(),
	)
	.await?;

	let response_options = expect_context::<ResponseOptions>();
	response_options.set_status(response.status_code);

	use_cookie_with_options::<AuthState, JsonSerdeCodec>(
		constants::AUTH_STATE,
		UseCookieOptions::default()
			.http_only(false)
			.secure(if cfg!(debug_assertions) { false } else { true }),
	)
	.1
	.set(Some(AuthState::LoggedIn {
		access_token: response.body.access_token,
		refresh_token: response.body.refresh_token,
		last_used_workspace_id: None,
	}));

	leptos_axum::redirect(
		if let Some(next) = next.as_deref() {
			next
		} else {
			"/"
		},
	);

	Ok(())
}

/// The login page component. This is the form that the user uses to log in to
/// the application.
#[allow(non_snake_case)]
pub fn LoginPage(query: LoginQuery, _: LoginRoute) -> impl IntoView {
	let LoginQuery { user_id, next } = query;

	let username = RwSignal::new(user_id.clone().unwrap_or_default());
	let password = RwSignal::new("".to_owned());

	let username_error = RwSignal::new("".to_owned());
	let password_error = RwSignal::new("".to_owned());

	let loading = RwSignal::new(false);

	let username_error_alert = move || {
		username_error.get().some_if_not_empty().map(|val| {
			view! {
				<Alert r#type={AlertType::Error} class="mt-xs">
					{val}
				</Alert>
			}
		})
	};

	let password_error_alert = move || {
		password_error.get().some_if_not_empty().map(|val| {
			view! {
				<Alert r#type={AlertType::Error} class="mt-xs">
					{val}
				</Alert>
			}
		})
	};

	view! {
		<ActionForm action={ServerAction::<LoginAction>::new()} attr:class="box-onboard text-white">
			<div class="flex justify-between items-baseline mb-lg w-full">
				<h1 class="text-primary text-xl text-medium">"Sign In"</h1>
				<div class="text-white text-thin flex items-start justify-start text-sm">
					<p>"New User? "</p>
					<Link to={"/sign-up".to_owned()}>
						"Sign Up"
					</Link>
				</div>
			</div>

			<div class="flex flex-col items-start justify-start w-full gap-md">
				<Input
					value={username}
					on_input={move |ev: Event| {
						username.set(event_target_value(&ev));
					}}
					id="user_id"
					name="user_id"
					class="w-full"
					r#type={InputType::Text}
					placeholder="Username / Email"
					disabled={false}
					start_icon={|| view! {
						<Icon
							icon={IconType::User}
							size={Size::ExtraSmall}
						/>
					}}
				/>

				{username_error_alert}

				<PasswordInput
					on_input={move |ev| {
						password.set(event_target_value(&ev));
					}}
					class="w-full"
					id="password"
					name="password"
					placeholder="Password"
					start_icon={|| view! {
						<Icon
							icon={IconType::Shield}
							size={Size::ExtraSmall}
						/>
					}}
				/>

				<input name="mfa_otp" type="hidden" />

				<input name="next" type="hidden" value={next.clone()} />

				{password_error_alert}
			</div>


			<Show
				when={move || !loading.get()}
				fallback={|| view! {
					<p>"Loading..."</p>
				}}
			>
				<Button
					r#type={ButtonType::Submit}
					class="btn ml-auto mt-md"
					variant={LinkStyleVariant::Contained}
				>
					"LOGIN"
				</Button>
			</Show>
		</ActionForm>
	}
}
