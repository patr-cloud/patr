use models::api::auth::{
	CompleteSignUpPath,
	CompleteSignUpRequest,
	CompleteSignUpRequestHeaders,
	CompleteSignUpResponse,
	LoginResponse,
};

use crate::prelude::*;

#[component]
pub fn ConfirmSignUpPage() -> impl IntoView {
	view! {
		<PageContainer class="bg-image">
			<ConfirmSignUpForm />
		</PageContainer>
	}
}

#[server(ConfirmOtp, endpoint = "auth/join")]
async fn complete_sign_up(
	username: String,
	otp: String,
) -> Result<Result<CompleteSignUpResponse, ErrorType>, ServerFnError> {
	Ok(make_api_call::<CompleteSignUpRequest>(
		ApiRequest::builder()
			.path(CompleteSignUpPath)
			.query(())
			.headers(CompleteSignUpRequestHeaders {
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(CompleteSignUpRequest {
				username,
				verification_token: otp,
			})
			.build(),
	)
	.await
	.map(|res| res.body))
}

#[component]
pub fn ConfirmSignUpForm() -> impl IntoView {
	let confirm_action = create_server_action::<ConfirmOtp>();

	let otp_error = create_rw_signal("".to_owned());
	let username_error = create_rw_signal("".to_owned());

	let response = confirm_action.value();
	// let has_error = move || response.with(|resp| matches!(resp, Some(Err(_))));

	let handle_errors = move |error: ErrorType| match error {
		ErrorType::UserNotFound => {
			username_error.set("User Not Found".to_owned());
		}
		ErrorType::MfaOtpInvalid => {
			otp_error.set("Invalid OTP".to_owned());
		}
		ErrorType::InternalServerError(err) => {
			otp_error.set(err.to_string());
		}
		e => {
			otp_error.set(format!("{:?}", e));
		}
	};

	create_effect(move |_| {
		if let Some(Ok(resp)) = response.get() {
			let _ = match resp {
				Ok(CompleteSignUpResponse {
					refresh_token,
					access_token,
				}) => {
					logging::log!("{}, {}", refresh_token, access_token);
					return;
				}
				Err(err) => {
					logging::log!("{:#?}", err);
					handle_errors(err);
					return;
				}
			};
		}
	});

	view! {
		<div class="box-onboard txt-white">
			<div class="fr-sb-bl mb-lg full-width">
				<h1 class="txt-primary txt-xl txt-medium">
					"Confirm OTP"
				</h1>

				<div class="txt-primary txt-thin fr-fs-fs">
					<Link
						to="/sign-up"
						r#type=Variant::Link
						class="ml-xs"
					>
						"Sign Up with different Email"
					</Link>
				</div>
			</div>

			<ActionForm action=confirm_action class="fc-fs-fs full-width">
				<Input
					name="username"
					placeholder="Username"
					id="username"
					class="full-width"
					r#type=InputType::Text
					required=true
				/>
				// <Show
				// 	when=move || !username_error.with(|error| error.is_empty())
				// 	fallback=|| view! {}.into_view()
				// >
				// 	// <Alert r#type=AlertType::Error class="mt-xs">{move || username_error.get()}</Alert>
				// 	<p>{username_error}</p>
				// </Show>
				<p>{username_error}</p>

				<span class="mt-sm mb-xxs txt-sm txt-white">"Enter OTP"</span>
				<Input
					name="otp"
					placeholder="Enter the 6 Digit OTP"
					id="username"
					class="full-width"
					r#type=InputType::Number
					required=true
				/>
				// <Show
				// 	when=move || !otp_error.with(|error| error.is_empty())
				// 	fallback=|| view! {}.into_view()
				// >
				// 	<p>{otp_error}</p>
				// 	// <Alert r#type=AlertType::Error class="mt-xs">{move || otp_error.get()}</Alert>
				// </Show>
				<p>{otp_error}</p>
				// {
				// 	move || {
				// 		otp_error.get().some_if_not_empty()
				// 		.map(|error| {
				// 			view! {
				// 				<Alert r#type=AlertType::Error class="mt-xs">{error}</Alert>
				// 			}
				// 		})
				// 	}
				// }

				<div class="fr-fe-ct full-width mt-lg">
					<Link
						should_submit=true
						r#type=Variant::Button
						style_variant=LinkStyleVariant::Contained
						class="btn mr-xs"
					>
						"SIGN UP"
					</Link>
				</div>
			</ActionForm>
		</div>
	}
}
