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
	logging::log!("{}, {}, {:?}", user_id, password, mfa_otp);
	Ok(make_api_call::<LoginRequest>(
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
	.await
	.map(|res| res.body))
}

#[component]
pub fn LoginForm() -> impl IntoView {
	let login_action = create_server_action::<Login>();

	view! {
		<ActionForm action=login_action class="box-onboard txt-white">
			<div class="fr-sb-bl mb-lg full-width">
				<h1 class="txt-primary txt-xl txt-medium">"Sign In"</h1>
				<div class="txt-white txt-thin fr-fs-fs">
					<p>"New User? "</p>
					<Link to="/sign-up" r#type=Variant::Link>
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

				<input name="mfa_otp" r#type="hidden" />
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
				r#type=Variant::Button
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
