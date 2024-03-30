use leptos::html::Header;
use leptos_router::ActionForm;
use models::api::auth::*;

use crate::prelude::*;

/// NameRequest, NameRequestHeader, NamePath, NameResponse
#[server(Login, endpoint = "auth/sign-in")]
async fn login(
	user_id: String,
	password: String,
	mfa_otp: Option<String>,
) -> Result<Result<LoginResponse, ErrorType>, ServerFnError> {
	use axum::{
		http::header::{HeaderValue, LOCATION, SET_COOKIE},
		response::AppendHeaders,
	};
	use axum_extra::extract::cookie::{Cookie, SameSite};
	use http::StatusCode;
	use leptos_axum::{redirect, ResponseOptions};
	use time::Duration;

	let api_response = make_api_call::<LoginRequest>(
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
	.await;
	let response = expect_context::<ResponseOptions>();

	if let Ok(ref resp) = api_response {
		// let mut cookie = Cookie::new("access_token");
		let access_cookie = Cookie::build(("access_token", resp.body.access_token.clone()))
			.path("/")
			.max_age(Duration::days(90))
			.same_site(SameSite::Lax)
			.build();
		let refresh_cookie = Cookie::build(("refresh_token", resp.body.refresh_token.clone()))
			.path("/")
			.max_age(Duration::days(90))
			.same_site(SameSite::Lax)
			.build();
		let access_token_header = HeaderValue::from_str(access_cookie.to_string().as_str());
		let refresh_token_header = HeaderValue::from_str(refresh_cookie.to_string().as_str());
		let redirect_header = HeaderValue::from_str("/some");

		if let (Ok(access_token_header), Ok(refresh_token_header), Ok(redirect_header)) =
			(access_token_header, refresh_token_header, redirect_header)
		{
			response.append_header(SET_COOKIE, access_token_header);
			response.append_header(SET_COOKIE, refresh_token_header);
			// response.append_header(LOCATION, redirect_header);
			redirect("/");
		}
	}

	Ok(api_response.map(|res| res.body))
}

#[component]
pub fn LoginForm() -> impl IntoView {
	let login_action = create_server_action::<Login>();
	let response = login_action.value();

	let username_error = create_rw_signal("".to_owned());
	let password_error = create_rw_signal("".to_owned());

	// let (auth_state, set_auth_state) = get_auth_state();

	let handle_errors = move |error: ErrorType| match error {
		ErrorType::UserNotFound => {
			username_error.set("User Not Found".to_owned());
		}
		ErrorType::InvalidPassword => {
			password_error.set("Invalid OTP".to_owned());
		}
		ErrorType::InternalServerError(err) => {
			password_error.set(err.to_string());
		}
		e => {
			password_error.set(format!("{:?}", e));
		}
	};

	create_effect(move |_| {
		if let Some(Ok(resp)) = response.get() {
			let _ = match resp {
				Ok(LoginResponse {
					access_token,
					refresh_token,
				}) => {
					logging::log!("{} {}", access_token, refresh_token);
					// set_auth_state.set(Some(AuthTokens {
					// 	refresh_token,
					// 	auth_token: access_token,
					// }))
				}
				Err(err) => {
					handle_errors(err);
					// logging::log!("{:#?}", err);
					return;
				}
			};
		}
	});

	// create_effect(move |_| logging::log!("{:#?}", auth_state.get()));

	view! {
		<ActionForm action=login_action class="box-onboard txt-white">
			<div class="fr-sb-bl mb-lg full-width">
				<h1 class="txt-primary txt-xl txt-medium">"Sign In"</h1>
				<div class="txt-white txt-thin fr-fs-fs">
					<p>"New User? "</p>
					<Link to="/sign-up".to_owned() r#type=Variant::Link>
						"Sign Up"
					</Link>
				</div>
			</div>

			<div class="fc-fs-fs full-width gap-md">
				<Input
					name="user_id"
					class="full-width"
					id="user_id"
					r#type=InputType::Text
					placeholder="Username/Email"
					start_icon=Some(
						IconProps::builder().icon(IconType::User).size(Size::ExtraSmall).build(),
					)
				/>
				<p>{username_error}</p>

				<Input
					name="password"
					class="full-width"
					id="password"
					r#type=InputType::Password
					placeholder="Password"
					start_icon=Some(
						IconProps::builder().icon(IconType::Shield).size(Size::ExtraSmall).build(),
					)
				/>

				<input name="mfa_otp" type="hidden" />
				<p>{password_error}</p>
			</div>

			<div class="fr-sb-ct full-width pt-xs">
				<Link
					to="/forgot-password".to_owned()
					r#type=Variant::Link
				>
					"Forgot Password?"
				</Link>
			</div>
			<Link
				should_submit=true
				r#type=Variant::Link
				class="btn ml-auto mt-md"
				style_variant=LinkStyleVariant::Contained
			>
				"LOGIN"
			</Link>
		</ActionForm>
	}
}

#[component]
pub fn AuthPage() -> impl IntoView {
	view! {
		<PageContainer class="bg-image">
			<Outlet />
		</PageContainer>
	}
}
