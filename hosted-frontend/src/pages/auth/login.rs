use leptos_router::ActionForm;
use models::api::auth::*;

use crate::prelude::*;

/// The login form component. This is the form that the user uses to log in to
/// the application.
#[component]
pub fn LoginForm() -> impl IntoView {
	let login_action = create_server_action::<Login>();
	let response = login_action.value();
	let AuthStateContext(context) = expect_context::<crate::utils::AuthStateContext>();

	let username_error = create_rw_signal("".to_owned());
	let password_error = create_rw_signal("".to_owned());

	// let global_state = expect_context::<RwSignal<AuthState>>();

	let handle_errors = move |error| match error {
		ServerFnError::WrappedServerError(ErrorType::UserNotFound) => {
			username_error.set("User Not Found".to_owned());
			password_error.set("".to_owned());
		}
		ServerFnError::WrappedServerError(ErrorType::InvalidPassword) => {
			username_error.set("".to_owned());
			password_error.set("Wrong Password".to_owned());
		}
		e => {
			username_error.set("".to_owned());
			password_error.set(e.to_string());
		}
	};

	create_effect(move |_| {
		if let Some(resp) = response.get() {
			match resp {
				Ok(LoginResponse {
					access_token,
					refresh_token,
				}) => {
					let auth_state = AuthState::LoggedIn {
						access_token,
						refresh_token,
						last_used_workspace_id: None,
					};
					auth_state.clone().save();
					context.set(auth_state);
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
		}
	});

	view! {
		<ActionForm action={login_action} class="box-onboard txt-white">
			<div class="fr-sb-bl mb-lg full-width">
				<h1 class="txt-primary txt-xl txt-medium">"Sign In"</h1>
				<div class="txt-white txt-thin fr-fs-fs">
					<p>"New User? "</p>
					<Link to={"/sign-up".to_owned()} r#type={Variant::Link}>
						"Sign Up"
					</Link>
				</div>
			</div>

			<div class="fc-fs-fs full-width gap-md">
				<Input
					name="user_id"
					class="full-width"
					id="user_id"
					r#type={InputType::Text}
					placeholder="Username/Email"
					start_icon={Some(
						IconProps::builder().icon(IconType::User).size(Size::ExtraSmall).build(),
					)}
				/>

				<Show when={move || !username_error.get().is_empty()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						{move || username_error.get()}
					</Alert>
				</Show>

				<Input
					name="password"
					class="full-width"
					id="password"
					r#type={InputType::Password}
					placeholder="Password"
					start_icon={Some(
						IconProps::builder().icon(IconType::Shield).size(Size::ExtraSmall).build(),
					)}
				/>

				<input name="mfa_otp" type="hidden"/>
				<Show when={move || !password_error.get().is_empty()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						{move || password_error.get()}
					</Alert>
				</Show>
			</div>

			<div class="fr-sb-ct full-width pt-xs">
				<Link to={"/forgot-password".to_owned()} r#type={Variant::Link}>
					"Forgot Password?"
				</Link>
			</div>
			<Link
				should_submit=true
				r#type={Variant::Button}
				class="btn ml-auto mt-md"
				style_variant={LinkStyleVariant::Contained}
			>
				"LOGIN"
			</Link>
		</ActionForm>
	}
}
