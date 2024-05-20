use leptos_use::{use_cookie, utils::FromToStringCodec};
use models::api::user::GetUserInfoResponse;

use crate::prelude::*;

/// Load user data from the server
#[server]
pub async fn load_user_data(
	access_token: Option<String>,
) -> Result<Result<GetUserInfoResponse, ErrorType>, ServerFnError> {
	use std::str::FromStr;

	use models::api::user::{GetUserInfoPath, GetUserInfoRequest, GetUserInfoRequestHeaders};

	logging::log!("Bearer {}", access_token.clone().unwrap_or_default());
	let api_response = make_api_call::<GetUserInfoRequest>(
		ApiRequest::builder()
			.path(GetUserInfoPath)
			.query(())
			.headers(GetUserInfoRequestHeaders {
				authorization: BearerToken::from_str(
					format!("Bearer {}", access_token.unwrap_or_default()).as_str(),
				)?,
				user_agent: UserAgent::from_static("hyper/0.12.2"),
			})
			.body(GetUserInfoRequest)
			.build(),
	)
	.await;

	Ok(api_response.map(|res| res.body))
}

#[component]
pub fn ProfileSettings() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>("access_token");
	let access_token_signal = move || access_token.get();
	let user_data = create_resource(access_token_signal, move |value| async move {
		load_user_data(value).await
	});

	view! {
		<div
			class="full-width fit-wide-screen mx-auto fc-fs-fs px-md my-xl gap-lg"
		>
			<Transition>
				{
					move || match user_data.get() {
						Some(Ok(user_data)) => {
							logging::log!("{:#?}", user_data);
							match user_data {
								Ok(data) => view! {
									<BasicInfo basic_user_info=data.clone().basic_user_info />
									<ContactInfo user_email=data.clone().recovery_email />
								}
								.into_view(),
								Err(_) => {
									view! {}.into_view()
								}
							}
						},
						Some(Err(_)) => {
							view! {}.into_view()
						},
						None => {
							view! {}.into_view()
						}
					}
				}
			</Transition>
			<PasswordSection />
		</div>
	}
}

mod basic_info;
mod contact_info;
mod email_card;
mod password_section;

pub use self::{basic_info::*, contact_info::*, email_card::*, password_section::*};
