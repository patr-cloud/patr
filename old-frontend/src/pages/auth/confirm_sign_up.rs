use models::api::auth::CompleteSignUpResponse;

use crate::prelude::*;

/// This page is shown to the user when they have signed up and need to confirm
/// their OTP to complete the sign-up process.
#[component]
pub fn ConfirmSignUpPage() -> impl IntoView {
	let (_, set_auth_state) = AuthState::load();
	let confirm_action = create_server_action::<ConfirmOtp>();

	let otp_error = create_rw_signal("".to_owned());
	let username_error = create_rw_signal("".to_owned());
	let otp = create_rw_signal("".to_string());

	let pending = confirm_action.pending();
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
					set_auth_state.set(Some(AuthState::LoggedIn {
						access_token,
						refresh_token,
						last_used_workspace_id: None,
					}));
				}
				Err(err) => {
					logging::log!("{:#?}", err);
					handle_errors(err);
				}
			}
		}
	});

	view! {
		<div class="box-onboard text-white">
			<div class="flex justify-between items-baseline mb-lg w-full">
				<h1 class="text-primary text-xl text-medium">"Confirm OTP"</h1>

				<div class="text-primary text-thin flex items-start justify-start">
					<Link to="/sign-up" r#type={Variant::Link} class="ml-xs">
						"Sign Up with different Email"
					</Link>
				</div>
			</div>

			<ActionForm
				action={confirm_action}
				class="flex flex-col items-start justify-start w-full"
			>
				<Input
					name="username"
					placeholder="Username"
					id="username"
					class="w-full"
					r#type={InputType::Text}
					required=true
				/>
				<Show when={move || !username_error.get().is_empty()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						{move || username_error.get()}
					</Alert>
				</Show>

				<span class="mt-sm mb-xxs text-sm text-white">"Enter OTP"</span>
				<input name="otp" type="hidden" value={otp} />
				<OtpInput otp={otp} on_change={move |val: String| otp.set(val)} />
				<Show when={move || !otp_error.get().is_empty()}>
					<Alert r#type={AlertType::Error} class="mt-xs">
						{move || otp_error.get()}
					</Alert>
				</Show>

				<div class="flex justify-end items-center w-full mt-lg">
					<Show
						when=move || !pending.get()
						fallback={|| view! {
							<Link
								disabled={true}
								r#type={Variant::Button}
								style_variant={LinkStyleVariant::Contained}
								class="btn mr-xs"
							>
								"LOADING"
							</Link>
						}}
					>
						<Link
							should_submit=true
							r#type={Variant::Button}
							style_variant={LinkStyleVariant::Contained}
							class="btn mr-xs"
						>
							"SIGN UP"
						</Link>
					</Show>
				</div>
			</ActionForm>
		</div>
	}
}
