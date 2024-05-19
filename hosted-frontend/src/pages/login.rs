use leptos_router::ActionForm;
use models::{api::auth::*, utils::Headers};

use crate::prelude::*;

#[server(Login, "/api/auth/login")]
async fn login(request: LoginRequest) -> Result<LoginResponse, ServerFnError<ErrorType>> {
	use crate::utils::make_api_call;
	let parts = leptos_axum::extract::<http::request::Parts>()
		.await
		.map_err(|err| {
			ServerFnError::ServerError(format!("Failed to extract request parts: {}", err))
		})?;

	let response = make_api_call(
		ApiRequest::<LoginRequest>::builder()
			.body(request)
			.headers(
				LoginRequestHeaders::from_header_map(&parts.headers)
					.map_err(|err| ServerFnError::ServerError(err.to_string()))?,
			)
			.path(LoginPath)
			.query(())
			.build(),
	)
	.await?;

	Ok(response.body)
}

#[component]
fn LoginForm() -> impl IntoView {
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
					class="full-width"
					id="username"
					r#type=InputType::Email
					placeholder="Username/Email"
					start_icon=Some(
						IconProps::builder().icon(IconType::User).size(Size::ExtraSmall).build(),
					)
				/>

				<Input
					class="full-width"
					id="password"
					r#type=InputType::Password
					placeholder="Password"
					start_icon=Some(
						IconProps::builder().icon(IconType::Shield).size(Size::ExtraSmall).build(),
					)
				/>

			</div>

			<div class="fr-sb-ct full-width pt-xs">
				<Link
					to="https://book.leptos.dev/view/03_components.html".to_owned()
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
pub fn LoginPage() -> impl IntoView {
	view! {
		<PageContainer class="bg-image">
			<LoginForm/>
		</PageContainer>
	}
}
