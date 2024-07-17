use models::api::auth::CompleteSignUpResponse;

use crate::prelude::*;

/// This page is shown to the user when they have signed up and need to confirm
/// their OTP to complete the sign-up process.
#[component]
pub fn ConfirmSignUpPage() -> impl IntoView {
	let confirm_action = create_server_action::<ConfirmOtp>();

	let otp_error = create_rw_signal("".to_owned());
	let username_error = create_rw_signal("".to_owned());

	let response = confirm_action.value();

	let handle_errors = move |error| match error {
		ServerFnError::WrappedServerError(ErrorType::UserNotFound) => {
			username_error.set("User Not Found".to_owned());
		}
		ServerFnError::WrappedServerError(ErrorType::MfaOtpInvalid) => {
			otp_error.set("Invalid OTP".to_owned());
		}
		e => {
			otp_error.set(e.to_string());
		}
	};

	create_effect(move |_| {
		if let Some(resp) = response.get() {
			match resp {
				Ok(CompleteSignUpResponse {
					refresh_token,
					access_token,
				}) => {
					let auth_state = AuthState::LoggedIn {
						access_token,
						refresh_token,
						last_used_workspace_id: None,
					};
					auth_state.clone().save();
					expect_context::<RwSignal<_>>().set(auth_state);
				}
				Err(err) => {
					logging::log!("{:#?}", err);
					handle_errors(err);
				}
			}
		}
	});

	view! {
		<div class="box-onboard txt-white">
			<div class="fr-sb-bl mb-lg full-width">
				<h1 class="txt-primary txt-xl txt-medium">"Confirm OTP"</h1>

				<div class="txt-primary txt-thin fr-fs-fs">
					<Link to="/sign-up" r#type={Variant::Link} class="ml-xs">
						"Sign Up with different Email"
					</Link>
				</div>
			</div>

			<ActionForm action={confirm_action} class="fc-fs-fs full-width">
				<Input
					name="username"
					placeholder="Username"
					id="username"
					class="full-width"
					r#type={InputType::Text}
					required=true
				/>
				<Show when={move || !username_error.get().is_empty()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						{move || username_error.get()}
					</Alert>
				</Show>

				<span class="mt-sm mb-xxs txt-sm txt-white">"Enter OTP"</span>
				<Input
					name="otp"
					placeholder="Enter the 6 Digit OTP"
					id="username"
					class="full-width"
					r#type={InputType::Number}
					required=true
				/>
				<Show when={move || !otp_error.get().is_empty()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						{move || otp_error.get()}
					</Alert>
				</Show>

				<div class="fr-fe-ct full-width mt-lg">
					<Link
						should_submit=true
						r#type={Variant::Button}
						style_variant={LinkStyleVariant::Contained}
						class="btn mr-xs"
					>
						"SIGN UP"
					</Link>
				</div>
			</ActionForm>
		</div>
	}
}
