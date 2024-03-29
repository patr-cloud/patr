use leptos::server_fn::redirect;
use models::api::auth::{
	CompleteSignUpPath,
	CompleteSignUpRequest,
	CompleteSignUpRequestHeaders,
	CompleteSignUpResponse,
	LoginResponse,
};

use crate::{
	global_state::{get_auth_state, set_auth_token, AuthTokens},
	prelude::*,
};

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
	use axum::{
		http::header::{HeaderValue, SET_COOKIE},
		response::AppendHeaders,
	};
	// use axum_extra::extract::cookie::Cookie;
	use http::StatusCode;
	use leptos_axum::ResponseOptions;

	let api_response = make_api_call::<CompleteSignUpRequest>(
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
	.await;

	let response = expect_context::<ResponseOptions>();

	if let Ok(resp) = &api_response {
		logging::log!("{:#?}", resp.body);
		// let mut cookie = Cookie::new("access_token");
		let access_token_header = HeaderValue::from_str("access_token=something");
		let refresh_token_header = HeaderValue::from_str("refresh_token=now");

		if let (Ok(access_token_cookie), Ok(refresh_token_cookie)) =
			(access_token_header, refresh_token_header)
		{
			response.append_header(SET_COOKIE, access_token_cookie);
			response.append_header(SET_COOKIE, refresh_token_cookie);
		}
		leptos_axum::redirect("/");
	}

	Ok(api_response.map(|res| res.body))
}

#[component]
pub fn ConfirmSignUpForm() -> impl IntoView {
	let confirm_action = create_server_action::<ConfirmOtp>();

	let otp_error = create_rw_signal("".to_owned());
	let username_error = create_rw_signal("".to_owned());

	let response = confirm_action.value();
	let (_, set_auth_state) = get_auth_state();
	// let set_auth_state = set_auth_token();
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
					set_auth_state.set(Some(AuthTokens {
						refresh_token,
						auth_token: access_token,
					}));
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
				<Show
					when=move || !username_error.with(|error| error.is_empty())
					fallback=|| view! {}.into_view()
				>
					// <Alert r#type=AlertType::Error class="mt-xs">{move || username_error.get()}</Alert>
					<p>{username_error}</p>
				</Show>
				// <p>{username_error}</p>

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
