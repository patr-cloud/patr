use models::{api::auth::*, frontend::auth::*};

use crate::prelude::*;

/// The server action to confirm the OTP and complete the sign-up process
#[server(ConfirmOtp)]
pub async fn confirm_action() -> Result<CompleteSignUpResponse, ServerFnError<ErrorType>> {
	todo!()
}

/// This page is shown to the user when they have signed up and need to confirm
/// their OTP to complete the sign-up process.
#[allow(non_snake_case)]
pub fn VerifySignUpPage(query: VerifySignUpQuery, _: VerifySignUpRoute) -> impl IntoView {
	let VerifySignUpQuery {
		user_id,
		signup_token,
	} = query;

	let auth_state = expect_context::<RwSignal<AuthState>>();
	let confirm_action = ServerAction::<ConfirmOtp>::new();

	let otp = RwSignal::new(signup_token.unwrap_or_default());
	let otp_error = RwSignal::new("".to_owned());
	let username_error = RwSignal::new("".to_owned());

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

	Effect::new(move |_| {
		if let Some(resp) = response.get() {
			match resp {
				Ok(CompleteSignUpResponse {
					refresh_token,
					access_token,
				}) => {
					auth_state.set(AuthState::LoggedIn {
						access_token,
						refresh_token,
					});
				}
				Err(err) => {
					log::warn!("{:#?}", err);
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
					<Link to="/sign-up" variant={LinkStyleVariant::Plain} class="ml-xs">
						"Sign Up with different Email"
					</Link>
				</div>
			</div>

			<ActionForm
				action={confirm_action}
				attr:class="flex flex-col items-start justify-start w-full"
			>
				<Input
					name="username"
					placeholder="Username"
					id="username"
					value={user_id.unwrap_or_default()}
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
				// TODO: <OtpInput otp={otp} on_change={move |val: String| otp.set(val)} />
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
								variant={LinkStyleVariant::Contained}
								class="btn mr-xs"
							>
								"LOADING"
							</Link>
						}}
					>
						<Button
							variant={LinkStyleVariant::Contained}
							class="btn mr-xs"
						>
							"SIGN UP"
						</Button>
					</Show>
				</div>
			</ActionForm>
		</div>
	}
}
