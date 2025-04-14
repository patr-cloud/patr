use ev::SubmitEvent;
use models::api::auth::LoginResponse;

use crate::prelude::*;

/// The API endpoint for logging in to the application. This endpoint is used to
/// authenticate the user and get the JWT tokens for the user.
#[server(LoginApi, endpoint = "auth/sign-in")]
pub async fn login(
	user_id: String,
	password: String,
	mfa_otp: Option<String>,
) -> Result<AuthState, ServerFnError<ErrorType>> {
	use std::str::FromStr;

	use models::api::{auth::*, user::*};

	let LoginResponse {
		access_token,
		refresh_token,
	} = make_api_call::<LoginRequest>(
		ApiRequest::builder()
			.path(LoginPath)
			.query(())
			.headers(LoginRequestHeaders {
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(LoginRequest {
				user_id,
				password,
				mfa_otp,
			})
			.build(),
	)
	.await?
	.body;

	let workspaces = make_api_call::<ListUserWorkspacesRequest>(
		ApiRequest::builder()
			.path(ListUserWorkspacesPath)
			.query(())
			.headers(ListUserWorkspacesRequestHeaders {
				authorization: BearerToken::from_str(&access_token)
					.map_err(|err| ServerFnError::<ErrorType>::ServerError(err.to_string()))?,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(ListUserWorkspacesRequest)
			.build(),
	)
	.await?
	.body
	.workspaces;

	let last_used_workspace_id = workspaces.into_iter().next().map(|workspace| workspace.id);

	// TODO: uncomment this when Leptos-use fixes this
	// set_state.set(Some(AuthState::LoggedIn {
	// 	access_token,
	// 	refresh_token,
	// 	last_used_workspace_id,
	// }));

	Ok(AuthState::LoggedIn {
		access_token,
		refresh_token,
		last_used_workspace_id,
	})
}

/// The login form component. This is the form that the user uses to log in to
/// the application.
#[component]
pub fn LoginForm(
	/// The query params for the page
	query: LoginQuery,
) -> impl IntoView {
	let LoginQuery { next, user_id } = query;

	let (_, set_state) = AuthState::load();
	let app_type = expect_context::<AppType>();

	let username = create_rw_signal(user_id.unwrap_or_default());
	let password = create_rw_signal("".to_owned());

	let username_error = create_rw_signal("".to_owned());
	let password_error = create_rw_signal("".to_owned());

	let loading = create_rw_signal(false);

	let on_submit_login = move |ev: SubmitEvent| {
		ev.prevent_default();
		loading.set(true);
		username_error.set("".to_string());
		password_error.set("".to_string());

		if username.get().is_empty() {
			username_error.set("Username / email cannot be empty".to_owned());
			loading.set(false);
			return;
		}

		if password.get().is_empty() {
			password_error.set("Password cannot be empty".to_owned());
			loading.set(false);
			return;
		}

		let next = next.clone();

		spawn_local(async move {
			match login(username.get_untracked(), password.get_untracked(), None).await {
				Ok(auth_state) => {
					set_state.set(Some(auth_state));
					use_navigate()(
						&next.unwrap_or_else(|| DeploymentsDashboardRoute {}.to_string()),
						NavigateOptions::default(),
					);
				}
				Err(ServerFnError::WrappedServerError(ErrorType::UserNotFound)) => {
					username_error.set("User Not Found".to_owned());
					password_error.set("".to_owned());
				}
				Err(ServerFnError::WrappedServerError(ErrorType::InvalidPassword)) => {
					username_error.set("".to_owned());
					password_error.set("Wrong Password".to_owned());
				}
				Err(ServerFnError::Deserialization(msg)) => {
					username_error.set("".to_owned());
					password_error.set(msg);
				}
				Err(err) => {
					username_error.set("".to_owned());
					password_error.set(err.to_string());
				}
			}

			loading.set(false);
		});
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
					id="user_id"
					name="user_id"
					class="w-full"
					r#type={InputType::Text}
					placeholder="Username / Email"
					disabled={loading}
					start_icon={Some(
						IconProps::builder().icon(IconType::User).size(Size::ExtraSmall).build(),
					)}
					on_input={Box::new(move |ev| {
						username.set(event_target_value(&ev));
					})}
					value={username}
				/>

				{move || username_error
					.get()
					.some_if_not_empty()
					.map(|message| view! {
						<Alert r#type={AlertType::Error} class="mt-xs">
							{&message}
						</Alert>
					})}

				<Input
					name="password"
					class="w-full"
					id="password"
					r#type={InputType::Password}
					placeholder="Password"
					start_icon={Some(
						IconProps::builder().icon(IconType::Shield).size(Size::ExtraSmall).build(),
					)}
					disabled={loading}
					on_input={Box::new(move |ev| {
						password.set(event_target_value(&ev));
					})}
					value={password}
				/>

				<input name="mfa_otp" type="hidden" />

				{move || password_error
					.get()
					.some_if_not_empty()
					.map(|message| view! {
						<Alert r#type={AlertType::Error} class="mt-xs">
							{&message}
						</Alert>
					})}
			</div>

			{app_type
				.is_managed()
				.then(|| {
					view! {
						<div class="flex justify-between items-center w-full pt-xs">
							<Link to={"/forgot-password".to_owned()} r#type={Variant::Link}>
								"Forgot Password?"
							</Link>
						</div>
					}
				})}

			{move || if loading.get() {
				view! {
					<Spinner class="ml-auto" />
				}
			} else {
				view! {
					<Link
						should_submit=true
						r#type={Variant::Button}
						class="btn ml-auto mt-md"
						style_variant={LinkStyleVariant::Contained}
						>
						"LOGIN"
					</Link>
				}
			}}
		</form>
	}
}
